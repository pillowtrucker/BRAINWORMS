pub mod prison_for_retarded_mozilla_dog_shit;

use std::{
    collections::HashMap, fs::File, io::Read, path::Path, ptr::slice_from_raw_parts, sync::Arc,
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
    Stop(JingleName),
    Drop(JingleName),
    Die,
}

pub type Jukebox = HashMap<JingleName, Vec<Stream<StereoFrame<f32>>>>;

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
            AudioCommand::Pause(_) => todo!(),
            AudioCommand::Stop(_) => todo!(),
            AudioCommand::Drop(n) => {
                let mut registry = registry.lock();
                registry.remove(&n);
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
/*
pub async fn play(filepath: &str) -> anyhow::Result<()> {
    let ctx = init("booger")?;
    let params = cubeb::StreamParamsBuilder::new()
        .format(STREAM_FORMAT)
        .rate(SAMPLE_FREQUENCY)
        .channels(2)
        .layout(cubeb::ChannelLayout::STEREO)
        .take();

    let mut position = 0u32;

    let mut builder = cubeb::StreamBuilder::<StereoFrame<f32>>::new();

    let mut file = File::open(filepath).unwrap();
    let mut buffer = Vec::new();
    let _ = file.read_to_end(&mut buffer).unwrap();

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

    let cv_playback_ended = Arc::new((Mutex::new(false), Condvar::new()));
    let cv_playback_ended_inside_copy = Arc::clone(&cv_playback_ended);

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
    let out_length = out_l.len();
    builder
        .name("Cubeb stereo")
        .default_output(&params)
        .latency(0x1000)
        .data_callback(move |_, output| {
            for f in output.iter_mut() {
                if (position as usize) < out_length {
                    f.l = out_l[position as usize];
                    f.r = out_r[position as usize];
                    position += 1;
                } else {
                    return 0;
                }
            }
            output.len() as isize
        })
        .state_callback(move |state| {
            println!("stream {:?}", state);
            match state {
                cubeb::State::Started => {}
                cubeb::State::Stopped => {}
                cubeb::State::Drained => {
                    let (lock, cvar) = &*cv_playback_ended_inside_copy;
                    let mut playback_ended = lock.lock();
                    *playback_ended = true;
                    cvar.notify_one();
                }
                cubeb::State::Error => panic!("playback error"),
            }
        });

    let stream = builder.init(&ctx).expect("Failed to create cubeb stream");

    stream.start()?;
    let (lock, cvar) = &*cv_playback_ended;
    cvar.wait_while(lock.lock().borrow_mut(), |&mut ended| !ended);
    Ok(())
}
*/
