use std::{
    fs::File,
    io::Read,
    ptr::slice_from_raw_parts,
    sync::{Arc, Condvar, Mutex},
};

use cubeb::{Context, Result, StereoFrame};
use libymfm::{driver::VgmPlay, sound::SoundSlot};
const SAMPLE_FREQUENCY: u32 = 48_000;
const STREAM_FORMAT: cubeb::SampleFormat = cubeb::SampleFormat::Float32LE;
pub fn init(ctx_name: &str) -> Result<Context> {
    let ctx_name = ustr::ustr(ctx_name);
    Context::init(Some(ctx_name.as_cstr()), None)
}
const MAX_SAMPLE_SIZE: usize = 2048;

pub async fn play(filepath: &str) -> Result<()> {
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
                    let mut playback_ended = lock.lock().unwrap();
                    *playback_ended = true;
                    cvar.notify_one();
                }
                cubeb::State::Error => panic!("playback error"),
            }
        });

    let stream = builder.init(&ctx).expect("Failed to create cubeb stream");

    stream.start()?;
    let (lock, cvar) = &*cv_playback_ended;
    let _guard = cvar
        .wait_while(lock.lock().unwrap(), |ended| !*ended)
        .unwrap();
    Ok(())
}
