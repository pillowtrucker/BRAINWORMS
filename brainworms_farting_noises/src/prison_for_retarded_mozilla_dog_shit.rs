use std::{
    borrow::BorrowMut,
    collections::hash_map::Entry::{Occupied, Vacant},
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

use cubeb::{Context, StereoFrame};
use parking_lot::{Condvar, Mutex};

use crate::{JingleName, JingleRegistry, Jukebox, SAMPLE_FREQUENCY, STREAM_FORMAT};

pub fn init(ctx_name: &str) -> anyhow::Result<Context> {
    let ctx_name = ustr::ustr(ctx_name);
    Ok(Context::init(Some(ctx_name.as_cstr()), None)?)
}
pub enum AudioPrisonOrder {
    Play(JingleName),
    Pause(JingleName),
    Stop(JingleName),
    Drop(JingleName),
    Die,
}
pub fn prison(
    gen: u64,
    rx: Receiver<AudioPrisonOrder>,
    registry: Arc<Mutex<JingleRegistry>>,
    tx: Sender<AudioPrisonOrder>,
) {
    let ctx = init(&format!("audio ctx gen {}", gen)).expect("failed context init");
    let mut state = Jukebox::new();
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioPrisonOrder::Play(name) => {
                println!("playing {name}");
                let registry = registry.lock();
                let jingle = &registry[&name];

                let out_length = jingle.len;
                let mut builder = cubeb::StreamBuilder::<StereoFrame<f32>>::new();
                let params = cubeb::StreamParamsBuilder::new()
                    .format(STREAM_FORMAT)
                    .rate(SAMPLE_FREQUENCY)
                    .channels(2)
                    .layout(cubeb::ChannelLayout::STEREO)
                    .take();
                let mut position = 0u32;

                let cv_playback_ended = Arc::new((Mutex::new(false), Condvar::new()));
                let cv_playback_ended_inside_copy = Arc::clone(&cv_playback_ended);
                let out_l = jingle.l.clone();
                let out_r = jingle.r.clone();
                builder
                    .name("Cubeb jingle {name}")
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
                let _ = stream.start();
                {
                    match state.entry(name.clone()) {
                        Occupied(mut eo) => {
                            eo.get_mut().push(stream);
                        }
                        Vacant(v) => {
                            v.insert(vec![stream]);
                        }
                    };
                }

                let tx = tx.clone();
                thread::spawn(move || {
                    let (lock, cvar) = &*cv_playback_ended;
                    cvar.wait_while(lock.lock().borrow_mut(), |&mut ended| !ended);
                    let _ = tx.send(AudioPrisonOrder::Drop(name));
                });
            }
            AudioPrisonOrder::Pause(_) => todo!(),
            AudioPrisonOrder::Stop(_) => todo!(),
            AudioPrisonOrder::Drop(name) => {
                state.remove(&name);
            }
            AudioPrisonOrder::Die => return,
        }
    }
}
