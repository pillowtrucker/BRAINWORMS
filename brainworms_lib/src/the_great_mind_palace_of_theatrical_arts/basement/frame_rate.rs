use std::time;

use super::input_handling::InputContext;

/// Sliding window to give a smooth framerate.
/// Sum the last `window_size` `frame_duration` to estimate the framerate.
/// Implemented with a circular buffer.
#[derive(Debug)]
pub struct FrameRate {
    /// Store the last frame durations.
    window: Vec<f32>,
    /// Index of the oldest frame duration,
    /// next frame duration will be stored here.
    current_index: usize,
}

impl FrameRate {
    /// Create a new slicing window with the given size.
    pub fn new(window_size: usize) -> Self {
        Self {
            current_index: 0,
            window: vec![0.0; window_size],
        }
    }

    /// Add the latest `frame_duration` to the window
    /// by remplacing the oldest `frame_duration`.
    pub fn update(&mut self, frame_duration: f32) {
        self.window[self.current_index] = frame_duration;
        self.current_index = (self.current_index + 1) % self.window.len();
    }

    /// Compute current `frame_rate`
    /// Since the mean of frame duration is `sum(window) / window_size`
    /// The number of frame per seconds is `1 / sum(window) / window_size`
    /// ie `window_size / sum(window)`
    pub fn get(&self) -> f32 {
        self.window.len() as f32 / self.window.iter().sum::<f32>()
    }

    /// Return current parity of the frame.
    /// This is useful for ping pong textures or buffers.
    #[allow(dead_code)]
    pub fn get_parity(&self) -> bool {
        self.current_index % 2 == 0
    }
}

impl Default for FrameRate {
    /// Create a default `FrameRate` with a window size of 20.
    fn default() -> Self {
        Self::new(20)
    }
}

pub fn update_frame_stats<InputContextEnum: InputContext>(
    state: &mut crate::GameProgrammeState<InputContextEnum>,
) {
    let last_update = state.last_update.unwrap();
    let last_frame_duration = last_update.elapsed().as_secs_f32();
    state.frame_rate.update(last_frame_duration);
    state.last_update = Some(time::Instant::now());
}