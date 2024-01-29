pub mod prison_for_retarded_mozilla_dog_shit;

use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Read,
    path::Path,
    ptr::slice_from_raw_parts,
    sync::Arc,
    thread,
};

use cubeb::Stream;
pub use cubeb::{self, Context, StereoFrame};
use libymfm::{driver::VgmPlay, sound::SoundSlot};

use parking_lot::Mutex;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::prison_for_retarded_mozilla_dog_shit::{prison, AudioPrisonOrder};
const SAMPLE_FREQUENCY: u32 = 48_000;
const STREAM_FORMAT: cubeb::SampleFormat = cubeb::SampleFormat::Float32LE;
const MAX_SAMPLE_SIZE: usize = 2048;

pub enum AudioCommand {
    Prebake(PathToJingle),
    Play(JingleName),
    Pause(JingleName),
    UnPause(JingleName),
    Drop(JingleName),
    Stop(JingleName),
    Die,
}

pub type Jukebox = HashMap<JingleName, VecDeque<Stream<StereoFrame<f32>>>>;

pub type JingleRegistry = HashMap<JingleName, Jingle>;
pub async fn audio_router_thread(
    mut rx: UnboundedReceiver<AudioCommand>,
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
            AudioCommand::Prebake(p) => {
                handle.spawn(async move { prebake(p, registry) });
            }
            AudioCommand::Play(name) => {
                println!("sending play command for {name} to prison");
                let _ = prison_tx.send(AudioPrisonOrder::Play(name));
            }
            AudioCommand::Pause(name) => {
                println!("pausing {name}");
                let _ = prison_tx.send(AudioPrisonOrder::Pause(name));
            }
            // this one drops the actual data
            AudioCommand::Drop(n) => {
                let mut registry = registry.lock();
                registry.remove(&n);
            }

            AudioCommand::UnPause(name) => {
                println!("unpausing {name}");
                let _ = prison_tx.send(AudioPrisonOrder::UnPause(name));
            }
            AudioCommand::Stop(name) => {
                println!("stopping {name}");
                let _ = prison_tx.send(AudioPrisonOrder::Pause(name.clone()));
                let _ = prison_tx.send(AudioPrisonOrder::Drop(name));
            }
            AudioCommand::Die => {
                let _ = prison_tx.send(AudioPrisonOrder::Die);
                let _ = prison_handle.join();
                return;
            }
        }
    }
}

pub type PathToJingle = String;
pub type JingleName = String;
#[derive(Debug, Clone, PartialEq)]
pub struct Jingle {
    pub name: JingleName,
    pub l: Arc<Vec<f32>>,
    pub r: Arc<Vec<f32>>,
    pub len: usize,
}

fn prebake(ptj: PathToJingle, registry: Arc<Mutex<JingleRegistry>>) -> anyhow::Result<()> {
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
    let jn = Path::new(&ptj)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    println!("added {jn} to registry");
    registry.insert(
        jn.clone(),
        Jingle {
            name: jn,
            l: out_l.into(),
            r: out_r.into(),
            len,
        },
    );

    Ok(())
}
