use std::{future::Future, sync::Arc};

use brainworms_arson::anyhow;
use glam::UVec2;
use parking_lot::Mutex;
use rend3::{types::SampleCount, Renderer, ShaderPreProcessor};
use rend3_routine::base::BaseRenderGraph;
use wgpu::{Features, Instance, PresentMode, Surface};
use winit::{
    error::EventLoopError,
    event_loop::{EventLoop, EventLoopBuilder, EventLoopWindowTarget},
    window::{Window, WindowBuilder},
};

use crate::{
    theater::{
        basement::{cla::GameProgrammeSettings, input_handling::InputContext},
        play::{
            scene::{actors::AstinkSprite, AstinkScene},
            Play, Playable,
        },
    },
    Event, GameProgramme, GameProgrammeData, GameProgrammeState, MyWinitEvent,
};

pub mod asset_loader;

pub struct DefaultRoutines {
    pub pbr: Mutex<rend3_routine::pbr::PbrRoutine>,
    pub skybox: Mutex<rend3_routine::skybox::SkyboxRoutine>,
    pub tonemapping: Mutex<rend3_routine::tonemapping::TonemappingRoutine>,
}
pub(crate) struct StoredSurfaceInfo {
    pub(crate) size: UVec2,
    pub(crate) scale_factor: f32,
    pub(crate) sample_count: SampleCount,
    pub(crate) present_mode: PresentMode,
}

impl<
        PlayablesEnum: Playable<InputContextEnum, PlayablesEnum> + 'static,
        InputContextEnum: InputContext,
    > GameProgramme<PlayablesEnum, InputContextEnum>
{
    pub fn start(self, window_builder: WindowBuilder) {
        {
            pollster::block_on(self.async_start(window_builder));
        }
    }

    pub fn spawn<Fut>(&self, fut: Fut) -> tokio::task::JoinHandle<<Fut as Future>::Output>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        self.rts.lock().as_ref().unwrap().spawn(fut)
    }

    pub fn new(play: Play<PlayablesEnum>) -> Self {
        let timestamp_start = Arc::new(std::time::Instant::now());
        let play = Arc::new(Mutex::new(play));
        let data = Arc::new(Mutex::new(GameProgrammeData {
            timestamp_start,
            play,
        }));
        Self {
            data,
            settings: Arc::new(Mutex::new(GameProgrammeSettings::new())),
            rts: Arc::new(Mutex::new(
                tokio::runtime::Builder::new_multi_thread().build().ok(),
            )),
            state: Arc::new(Mutex::new(GameProgrammeState::default())),
        }
    }

    pub(crate) fn sample_count(&self) -> rend3::types::SampleCount {
        self.settings.lock().samples
    }
    pub(crate) fn winit_run<F, T>(
        event_loop: winit::event_loop::EventLoop<T>,
        event_handler: F,
    ) -> Result<(), EventLoopError>
    where
        F: FnMut(winit::event::Event<T>, &EventLoopWindowTarget<T>) + 'static,
        T: 'static,
    {
        event_loop.run(event_handler)
    }

    pub(crate) fn scale_factor(&self) -> f32 {
        // Android has very low memory bandwidth, so lets run internal buffers at half
        // res by default
        cfg_if::cfg_if! {
            if #[cfg(target_os = "android")] {
                0.5
            } else {
                1.0
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_surface(
        &mut self,
        window: Arc<Window>,
        event: &Event,
        instance: &Instance,
        surface: &mut Option<Arc<Surface>>,
        renderer: &Arc<Renderer>,
        format: rend3::types::TextureFormat,
        surface_info: &mut StoredSurfaceInfo,
    ) -> Option<bool> {
        match *event {
            Event::Resumed => {
                if surface.is_none() {
                    // uhh this is still the same one line of unsafe I guess but for android
                    *surface = Some(Arc::new(instance.create_surface(window.clone()).unwrap()));
                }
                Some(false)
            }
            Event::Suspended => {
                *surface = None;
                Some(true)
            }
            Event::WindowEvent {
                event: winit::event::WindowEvent::Resized(size),
                ..
            } => {
                log::debug!("resize {:?}", size);
                let size = UVec2::new(size.width, size.height);

                if size.x == 0 || size.y == 0 {
                    return Some(false);
                }

                surface_info.size = size;
                surface_info.scale_factor = self.scale_factor();
                surface_info.sample_count = self.sample_count();
                surface_info.present_mode = self.present_mode();

                // Reconfigure the surface for the new size.
                rend3::configure_surface(
                    surface.as_ref().unwrap(),
                    &renderer.device,
                    format,
                    size,
                    surface_info.present_mode,
                );

                let alpha_mode = wgpu::CompositeAlphaMode::Auto;

                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    width: window.inner_size().width,
                    height: window.inner_size().height,
                    present_mode: wgpu::PresentMode::Immediate,
                    alpha_mode,
                    view_formats: Vec::new(),
                    desired_maximum_frame_latency: 2,
                };

                surface
                    .as_ref()
                    .unwrap()
                    .configure(&renderer.device, &config);

                // Tell the renderer about the new aspect ratio.
                renderer.set_aspect_ratio(size.x as f32 / size.y as f32);
                Some(false)
            }
            _ => None,
        }
    }
    pub(crate) fn create_iad<'a>(
        &'a mut self,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<rend3::InstanceAdapterDevice>> + 'a>,
    > {
        Box::pin(async move {
            let settings = self.settings.lock();
            Ok(rend3::create_iad(
                settings.desired_backend,
                settings.desired_device_name.clone(),
                settings.desired_profile,
                Some(Features::ADDRESS_MODE_CLAMP_TO_BORDER),
            )
            .await?)
        })
    }
    pub(crate) fn create_window(
        &mut self,
        builder: WindowBuilder,
    ) -> Result<(EventLoop<MyWinitEvent<AstinkScene, AstinkSprite>>, Window), EventLoopError> {
        profiling::scope!("creating window");

        let event_loop = EventLoopBuilder::with_user_event().build()?;
        let window = builder.build(&event_loop).expect("Could not build window");

        Ok((event_loop, window))
    }

    pub(crate) fn present_mode(&self) -> rend3::types::PresentMode {
        self.settings.lock().present_mode
    }
}
pub(crate) fn create_base_rendergraph(
    renderer: &Arc<Renderer>,
    spp: &ShaderPreProcessor,
) -> BaseRenderGraph {
    BaseRenderGraph::new(renderer, spp)
}
