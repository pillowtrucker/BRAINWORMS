use std::{
    borrow::BorrowMut,
    collections::{
        hash_map::Entry::{Occupied, Vacant},
        HashMap,
    },
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

use cubeb::{Context, StereoFrame};
use log::{error, info, warn};
use parking_lot::{Condvar, Mutex};

use crate::{JingleRegistry, Jukebox, SoundGroup, SoundVolume, SAMPLE_FREQUENCY, STREAM_FORMAT};

pub fn init(ctx_name: &str) -> anyhow::Result<Context> {
    let ctx_name = ustr::ustr(ctx_name);
    Ok(Context::init(Some(ctx_name.as_cstr()), None)?)
}
use crate::TicketedAudioRequestData as TARD;
pub enum AudioPrisonOrder {
    Play(TARD),
    Pause(TARD),
    UnPause(TARD),
    Drop(TARD),
    SetVolume(SoundGroup, SoundVolume),
    Die,
}
pub fn prison(
    gen: u64,
    rx: Receiver<AudioPrisonOrder>,
    registry: Arc<Mutex<JingleRegistry>>,
    tx: Sender<AudioPrisonOrder>,
) {
    let ctx = init(&format!("audio ctx gen {}", gen)).expect("failed audio context init");
    let mut state = Jukebox::new();
    while let Ok(cmd) = rx.recv() {
        match cmd {
            AudioPrisonOrder::Play(tard) => match tard {
                TARD::ByName(name) => {
                    error!("missing ticket uuid in Play request for {name}")
                }
                TARD::ByPath(p) => error!("illegal path argument in Play request for {p:?}"),

                TARD::Targeted(ref name, u) => {
                    info!("playing {name}");
                    let jingle;
                    let out_length;
                    let out_l;
                    let out_r;
                    let vol;
                    {
                        let reg = registry.lock();
                        jingle = &reg.jingles[name];
                        out_length = jingle.len;
                        out_l = jingle.l.clone();
                        out_r = jingle.r.clone();
                        vol = reg.volume[&jingle.group];
                    }

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

                    let n_dc = name.clone();
                    let n_sc = name.clone();
                    builder
                        .name(format!("Cubeb jingle {n_dc} instance {u}"))
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
                            info!("stream {:?} {n_sc}", state);
                            match state {
                                cubeb::State::Started => {}
                                cubeb::State::Stopped => {}
                                cubeb::State::Drained => {
                                    let (lock, cvar) = &*cv_playback_ended_inside_copy;
                                    let mut playback_ended = lock.lock();
                                    *playback_ended = true;
                                    cvar.notify_one();
                                }
                                cubeb::State::Error => error!("playback error {n_sc} instance {u}"),
                            }
                        });

                    let stream = builder
                        .init(&ctx)
                        .expect("Failed to create cubeb stream for {name}");
                    let _ = stream.set_volume(vol);
                    let _ = stream.start();
                    {
                        match state.entry(name.clone()) {
                            Occupied(mut eo) => {
                                eo.get_mut().insert(u, stream);
                            }
                            Vacant(v) => {
                                v.insert(HashMap::from([(u, stream)]));
                            }
                        };
                    }

                    let tx = tx.clone();
                    thread::spawn(move || {
                        let (lock, cvar) = &*cv_playback_ended;
                        cvar.wait_while(lock.lock().borrow_mut(), |&mut ended| !ended);
                        let _ = tx.send(AudioPrisonOrder::Drop(tard));
                    });
                }
            },
            AudioPrisonOrder::Pause(tard) => match tard {
                TARD::ByPath(p) => {
                    let n: String = p.file_name().unwrap().to_string_lossy().into();
                    warn!("Found path {p:?} instead of filename in Pause request, continuing with {n}");
                    let _ = tx.send(AudioPrisonOrder::Pause(TARD::ByName(n)));
                }
                TARD::ByName(name) => {
                    warn!("Pausing ALL instances of {name}");
                    state.get(&name).map(|m| m.iter().map(|(_, s)| s.stop()));
                }
                TARD::Targeted(n, u) => {
                    info!("pausing instance {u} of {n}");
                    state.get(&n).map(|m| m.get(&u).map(|s| s.stop()));
                }
            },
            // this one drops the stream but keeps data prebaked
            AudioPrisonOrder::Drop(tard) => {
                match tard {
                    TARD::ByPath(p) => {
                        let n: String = p.file_name().unwrap().to_string_lossy().into();
                        warn!("Found path {p:?} instead of filename in Drop request, continuing with {n}");
                        let _ = tx.send(AudioPrisonOrder::Drop(TARD::ByName(n)));
                    }
                    TARD::ByName(name) => {
                        warn!("Dropping ALL instances of {name}");
                        state.remove(&name);
                    }
                    TARD::Targeted(n, u) => {
                        info!("Dropping stream {n} instance {u}");
                        state.get_mut(&n).map(|m| m.remove(&u));
                    }
                }
            }
            AudioPrisonOrder::UnPause(tard) => match tard {
                TARD::ByPath(p) => {
                    let n: String = p.file_name().unwrap().to_string_lossy().into();
                    warn!("Found path {p:?} instead of filename in UnPause request, continuing with {n}");
                    let _ = tx.send(AudioPrisonOrder::UnPause(TARD::ByName(n)));
                }
                TARD::ByName(name) => {
                    warn!("Unpausing ALL instances of {name}");
                    state.get(&name).map(|m| m.iter().map(|(_, s)| s.start()));
                }
                TARD::Targeted(n, u) => {
                    info!("unpausing instance {u} of {n}");
                    state.get(&n).map(|m| m.get(&u).map(|s| s.start()));
                }
            },
            AudioPrisonOrder::SetVolume(group, volume) => {
                let mut reg = registry.lock();
                reg.volume.insert(group.clone(), volume);
                let affected_js: Vec<_> = reg
                    .jingles
                    .values()
                    .filter_map(|j| {
                        if j.group == group {
                            Some(j.name.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect();
                for (n, s) in &state {
                    if affected_js.contains(n) {
                        for s in s.values() {
                            let _ = s.set_volume(volume);
                        }
                    }
                }
            }
            AudioPrisonOrder::Die => return,
        }
    }
}
