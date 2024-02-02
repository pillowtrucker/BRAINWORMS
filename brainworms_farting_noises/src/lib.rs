pub mod prison;

use std::{
    collections::HashMap, fs::File, io::Read, path::PathBuf, ptr::slice_from_raw_parts, sync::Arc,
    thread,
};

use cubeb::Stream;
pub use cubeb::{self, Context, StereoFrame};
use libymfm::{driver::VgmPlay, sound::SoundSlot};
use log::{info, warn};
use parking_lot::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

use crate::prison::{prison, AudioPrisonOrder};
const SAMPLE_FREQUENCY: u32 = 48_000;
const STREAM_FORMAT: cubeb::SampleFormat = cubeb::SampleFormat::Float32LE;
const MAX_SAMPLE_SIZE: usize = 2048;
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum TicketedAudioRequestData {
    ByPath(PathToJingle),
    ByName(JingleName),
    Targeted(JingleName, Uuid),
}

use self::TicketedAudioRequestData as TARD;
impl From<AudioCommand> for AudioPrisonOrder {
    fn from(val: AudioCommand) -> Self {
        match val {
            AudioCommand::Prebake(_,_) => {
                panic!("illegal conversion attempt from {val:?} into AudioPrisonOrder")
            }
            AudioCommand::Play(d) => AudioPrisonOrder::Play(d),
            AudioCommand::Pause(d) => AudioPrisonOrder::Pause(d),
            AudioCommand::UnPause(d) => AudioPrisonOrder::UnPause(d),
            AudioCommand::Drop(_) => panic!("AudioCommand::Drop and AudioPrisonOrder::Drop have significantly different meaning. Encountered {val:?}"),
            AudioCommand::Stop(d) => AudioPrisonOrder::Drop(d),
            AudioCommand::Die => {
                warn!("I'm not sure you really wanted to do this - converting AudioCommand::Die into AudioPrisonOrder::Die");
                AudioPrisonOrder::Die
            },
            AudioCommand::SetVolume(g,v) => AudioPrisonOrder::SetVolume(g,v),
        }
    }
}
#[derive(Debug, Clone)]
pub enum AudioCommand {
    Prebake(TARD, SoundGroup),
    Play(TARD),
    Pause(TARD),
    UnPause(TARD),
    Drop(TARD),
    Stop(TARD),
    SetVolume(SoundGroup, SoundVolume),
    Die,
}
pub type VoiceID = String;
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SoundGroup {
    BGM,
    SFX,
    Voice(VoiceID),
}
// this seems to be logarithmic scale like decibels but normalized to [0,1] which is very cool and useful I guess if you are an alien studying human acoustics
pub type SoundVolume = f32;
pub type Jukebox = HashMap<JingleName, TicketMap>;
#[derive(Debug)]
pub struct JingleRegistry {
    pub jingles: HashMap<JingleName, Jingle>,
    pub volume: HashMap<SoundGroup, SoundVolume>,
}

impl Default for JingleRegistry {
    fn default() -> Self {
        Self {
            jingles: Default::default(),
            volume: HashMap::from([(SoundGroup::BGM, 0.1), (SoundGroup::SFX, 0.1)]),
        }
    }
}
pub type TicketMap = HashMap<Uuid, Stream<StereoFrame<f32>>>;
pub async fn audio_router_thread(
    mut rx: UnboundedReceiver<AudioCommand>,
    tx: UnboundedSender<AudioCommand>,
    registry: Arc<Mutex<JingleRegistry>>,
    gen: u64,
) {
    //    let mut state = Jukebox::new();
    let (prison_tx, prison_rx) = std::sync::mpsc::channel();
    let prison_registry = registry.clone();
    let prison_tx_theirs = prison_tx.clone();
    let prison_handle =
        thread::spawn(move || prison(gen, prison_rx, prison_registry, prison_tx_theirs));
    use tokio::runtime::Handle;
    while let Some(cmd) = rx.recv().await {
        let registry = registry.clone();
        let handle = Handle::current();
        match cmd {
            AudioCommand::Prebake(tard, group) => match tard {
                TicketedAudioRequestData::ByPath(p) => {
                    handle.spawn(async move {
                        info!("compiling {p:?}");
                        let _ = prebake(p, group, registry);
                    });
                }
                TicketedAudioRequestData::ByName(n) => {
                    warn!("prebake accepts paths only, illegal string argument {n}")
                }
                TicketedAudioRequestData::Targeted(n, u) => {
                    warn!("prebake accepts paths only, illegal string+uuid argument {n},{u}")
                }
            },
            AudioCommand::Play(tard) => {
                let _ = prison_tx.send(AudioPrisonOrder::Play(tard));
            }
            AudioCommand::Pause(tard) => {
                let _ = prison_tx.send(AudioPrisonOrder::Pause(tard));
            }
            // this one drops the actual data
            AudioCommand::Drop(tard) => {
                match tard {
                    TicketedAudioRequestData::ByPath(p) => {
                        let n: String = p.file_name().unwrap().to_string_lossy().into();
                        warn!("Found path {p:?} instead of filename in Drop request, continuing with {n}");
                        let _ = tx.send(AudioCommand::Drop(TARD::ByName(n)));
                    }
                    TicketedAudioRequestData::ByName(n) => {
                        let mut registry = registry.lock();
                        registry.jingles.remove(&n);
                    }
                    TicketedAudioRequestData::Targeted(n, u) => {
                        warn!("illegal uuid argument {u} to Drop - Send the file name {n} alone instead");
                    }
                }
            }

            AudioCommand::UnPause(tard) => {
                let _ = prison_tx.send(AudioPrisonOrder::UnPause(tard));
            }
            AudioCommand::Stop(ref tard) => {
                match tard {
                    TicketedAudioRequestData::ByPath(p) => {
                        let n: String = p.file_name().unwrap().to_string_lossy().into();
                        warn!("Found path {p:?} instead of filename in Stop request, continuing with {n}");
                        let _ = tx.send(AudioCommand::Stop(TARD::ByName(n)));
                    }
                    clean_one => {
                        info!("processing {:?}", clean_one);
                        let _ = prison_tx.send(AudioPrisonOrder::Pause(tard.to_owned()));
                        let _ = prison_tx.send(cmd.into());
                    }
                }
            }
            AudioCommand::Die => {
                warn!("Killing audio backend and audio data manager.");
                let _ = prison_tx.send(cmd.into());
                let _ = prison_handle.join();
                return;
            }
            AudioCommand::SetVolume(ref group, volume) => {
                info!("setting volume {}% for {group:?}", volume * 100.0);
                let _ = prison_tx.send(cmd.into());
            }
        }
    }
}

pub type PathToJingle = PathBuf;
pub type JingleName = String;
#[derive(Debug, Clone, PartialEq)]
pub struct Jingle {
    pub name: JingleName,
    pub l: Arc<Vec<f32>>,
    pub r: Arc<Vec<f32>>,
    pub len: usize,
    pub group: SoundGroup,
}

fn prebake(
    ptj: PathToJingle,
    group: SoundGroup,
    registry: Arc<Mutex<JingleRegistry>>,
) -> anyhow::Result<()> {
    let mut file = File::open(&ptj)?;
    let mut buffer = Vec::new();
    let _ = file.read_to_end(&mut buffer)?;

    // read vgm
    let mut vgmplay = VgmPlay::new(
        SoundSlot::new(SAMPLE_FREQUENCY, SAMPLE_FREQUENCY, MAX_SAMPLE_SIZE),
        &buffer,
    )
    .unwrap();
    let mut sampling_l;
    let mut sampling_r;

    let mut out_l = Vec::<f32>::with_capacity(MAX_SAMPLE_SIZE * 2);
    let mut out_r = Vec::<f32>::with_capacity(MAX_SAMPLE_SIZE * 2);

    #[allow(clippy::absurd_extreme_comparisons)]
    while vgmplay.play(false) <= 0 {
        unsafe {
            sampling_l = slice_from_raw_parts(vgmplay.get_sampling_l_ref(), MAX_SAMPLE_SIZE)
                .as_ref()
                .unwrap();
            sampling_r = slice_from_raw_parts(vgmplay.get_sampling_r_ref(), MAX_SAMPLE_SIZE)
                .as_ref()
                .unwrap();
        }
        out_l.extend_from_slice(sampling_l);
        out_r.extend_from_slice(sampling_r);
    }

    let len = out_l.len().max(out_r.len());
    let mut registry = registry.lock();
    let jn: String = ptj.file_name().unwrap().to_string_lossy().into();
    info!("added {jn} to registry");
    registry.jingles.insert(
        jn.clone(),
        Jingle {
            name: jn,
            l: out_l.into(),
            r: out_r.into(),
            len,
            group,
        },
    );

    Ok(())
}
