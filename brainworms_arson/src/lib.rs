#![feature(variant_count, exact_size_is_empty, array_chunks, iter_array_chunks)]

use std::{f32::consts::PI, mem::variant_count, sync::Arc};

pub use egui;
use egui::{
    egui_assert,
    epaint::{self, ClippedShape, Primitive, TextShape},
    lerp, pos2, vec2, Align, Color32, Direction, FontSelection, Galley, Pos2, Rect, Response,
    Sense, Shape, Stroke, Ui, Widget, WidgetInfo, WidgetText, WidgetType,
};
pub use egui_wgpu;
pub use egui_winit;
pub use nanorand;
use nanorand::{RandomGen, Rng};

pub struct KineticLabel<'a> {
    pub text: WidgetText,
    pub wrap: Option<bool>,
    pub truncate: bool,
    pub sense: Option<Sense>,
    pub kinesis: Option<Vec<&'a KineticEffect>>,
}
#[derive(Clone)]
pub enum KineticEffect {
    SineWavify { params: SineWavify },
    ShakeLetters { params: ShakeLetters },
    Gay { params: Gay },
}
impl<Generator: Rng<OUTPUT>, const OUTPUT: usize> RandomGen<Generator, OUTPUT> for KineticEffect {
    fn random(rng: &mut Generator) -> Self {
        match rng.generate_range(0..variant_count::<KineticEffect>()) {
            0 => KineticEffect::SineWavify {
                params: SineWavify::default(),
            },
            1 => KineticEffect::ShakeLetters {
                params: ShakeLetters::default(),
            },
            2 => KineticEffect::Gay {
                params: Gay::default(),
            },
            _ => KineticEffect::default(),
        }
    }
}
#[derive(Clone)]
pub struct Gay {
    pub rainbow: Vec<Color32>,
    pub live: bool,
    pub live_dampen: u64,
}
impl Default for Gay {
    fn default() -> Self {
        Self {
            rainbow: vec![
                Color32::RED,
                Color32::from_rgb(255, 127, 0),
                Color32::YELLOW,
                Color32::GREEN,
                Color32::BLUE,
                Color32::from_rgb(63, 0, 127),
                Color32::from_rgb(127, 0, 255),
            ],
            live: true,
            live_dampen: 20,
        }
    }
}
/// This is way too dependent on fps and screen dimensions and resolution to figure out good defaults
#[derive(Copy, Clone)]
pub struct ShakeLetters {
    pub max_distortion: i32,
    pub dampen: u64,
}
#[derive(Copy, Clone)]
pub struct SineWavify {
    pub amp: f32,
    pub x_0: f32,
    pub x_1: f32,
    pub live: bool,
    pub live_dampen: f32,
}

impl Default for ShakeLetters {
    fn default() -> Self {
        Self {
            max_distortion: 10,
            dampen: 8,
        }
    }
}
impl Default for SineWavify {
    fn default() -> Self {
        Self {
            amp: 10.0,
            x_0: 0.0,
            x_1: 2.0 * PI,
            live: true,
            live_dampen: 1000.0,
        }
    }
}
impl Default for KineticEffect {
    fn default() -> Self {
        Self::SineWavify {
            params: SineWavify::default(),
        }
    }
}

impl<'a> KineticLabel<'a> {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            wrap: None,
            truncate: false,
            sense: None,
            kinesis: None,
        }
    }

    #[inline]
    pub fn kinesis(mut self, kinesis: Vec<&'a KineticEffect>) -> Self {
        self.kinesis = Some(kinesis);
        self
    }
    #[allow(dead_code)]
    pub fn text(&self) -> &str {
        self.text.text()
    }

    /// If `true`, the text will wrap to stay within the max width of the [`Ui`].
    ///
    /// Calling `wrap` will override [`Self::truncate`].
    ///
    /// By default [`Self::wrap`] will be `true` in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and `false` on non-wrapping horizontal layouts.
    ///
    /// Note that any `\n` in the text will always produce a new line.
    ///
    /// You can also use [`egui::Style::wrap`].
    #[inline]
    #[allow(dead_code)]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self.truncate = false;
        self
    }

    /// If `true`, the text will stop at the max width of the [`Ui`],
    /// and what doesn't fit will be elided, replaced with `…`.
    ///
    /// If the text is truncated, the full text will be shown on hover as a tool-tip.
    ///
    /// Default is `false`, which means the text will expand the parent [`Ui`],
    /// or wrap if [`Self::wrap`] is set.
    ///
    /// Calling `truncate` will override [`Self::wrap`].
    #[inline]
    #[allow(dead_code)]
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.wrap = None;
        self.truncate = truncate;
        self
    }

    /// Make the label respond to clicks and/or drags.
    ///
    /// By default, a label is inert and does not respond to click or drags.
    /// By calling this you can turn the label into a button of sorts.
    /// This will also give the label the hover-effect of a button, but without the frame.
    ///
    /// ```
    /// # use egui::{Label, Sense};
    /// # egui::__run_test_ui(|ui| {
    /// if ui.add(Label::new("click me").sense(Sense::click())).clicked() {
    ///     /* … */
    /// }
    /// # });
    /// ```
    #[inline]
    #[allow(dead_code)]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }
}

impl KineticLabel<'_> {
    /// Do layout and position the galley in the ui, without painting it or adding widget info.
    pub fn layout_in_ui(&mut self, ui: &mut Ui) -> (Pos2, Arc<Galley>, Response) {
        let sense = self.sense.unwrap_or_else(|| {
            // We only want to focus labels if the screen reader is on.
            if ui.memory(|mem| mem.options.screen_reader) {
                Sense::focusable_noninteractive()
            } else {
                Sense::hover()
            }
        });
        if let WidgetText::Galley(galley) = &self.text {
            // If the user said "use this specific galley", then just use it:
            let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
            let pos = match galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };

            (pos, galley.clone(), response)
        } else {
            let valign = ui.layout().vertical_align();
            let mut lay_job =
                self.text
                    .to_owned()
                    .into_layout_job(ui.style(), FontSelection::Default, valign);

            let truncate = self.truncate;
            let wrap = !truncate && self.wrap.unwrap_or_else(|| ui.wrap_text());
            let available_width = ui.available_width();

            if wrap
                && ui.layout().main_dir() == Direction::LeftToRight
                && ui.layout().main_wrap()
                && available_width.is_finite()
            {
                // On a wrapping horizontal layout we want text to start after the previous widget,
                // then continue on the line below! This will take some extra work:

                let cursor = ui.cursor();
                let first_row_indentation = available_width - ui.available_size_before_wrap().x;
                egui_assert!(first_row_indentation.is_finite());

                lay_job.wrap.max_width = available_width;
                lay_job.first_row_min_height = cursor.height();
                lay_job.halign = Align::Min;
                lay_job.justify = false;
                if let Some(first_section) = lay_job.sections.first_mut() {
                    first_section.leading_space = first_row_indentation;
                }
                let galley = ui.fonts(|f| f.layout_job(lay_job));

                let pos = pos2(ui.max_rect().left(), ui.cursor().top());
                assert!(!galley.rows.is_empty(), "Galleys are never empty");
                // collect a response from many rows:
                let rect = galley.rows[0].rect.translate(vec2(pos.x, pos.y));
                let mut response = ui.allocate_rect(rect, sense);
                for row in galley.rows.iter().skip(1) {
                    let rect = row.rect.translate(vec2(pos.x, pos.y));
                    response |= ui.allocate_rect(rect, sense);
                }
                (pos, galley, response)
            } else {
                if truncate {
                    lay_job.wrap.max_width = available_width;
                    lay_job.wrap.max_rows = 1;
                    lay_job.wrap.break_anywhere = true;
                } else if wrap {
                    lay_job.wrap.max_width = available_width;
                } else {
                    lay_job.wrap.max_width = f32::INFINITY;
                };
                /*
                // is_grid is private but this might be important for embedding in grids
                if ui.is_grid() {
                    // TODO(emilk): remove special Grid hacks like these
                    text_job.job.halign = Align::LEFT;
                    text_job.job.justify = false;
                } else {
                    text_job.job.halign = ui.layout().horizontal_placement();
                    text_job.job.justify = ui.layout().horizontal_justify();
                };
                */
                lay_job.halign = ui.layout().horizontal_placement();
                lay_job.justify = ui.layout().horizontal_justify();
                let galley = ui.fonts(|f| f.layout_job(lay_job));
                let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
                let pos = match galley.job.halign {
                    Align::LEFT => rect.left_top(),
                    Align::Center => rect.center_top(),
                    Align::RIGHT => rect.right_top(),
                };
                (pos, galley, response)
            }
        }
    }
}

impl Widget for KineticLabel<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let (pos, galley, mut response) = self.layout_in_ui(ui);

        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, galley.text()));

        if galley.elided {
            // Show the full (non-elided) text on hover:

            let text = galley.text();
            response = response.clone().on_hover_ui(|ui| {
                ui.add(egui::Label::new(text));
            });
        }
        if !ui.is_rect_visible(response.rect) {
            return response;
        }

        let response_color = ui.style().interact(&response).text_color();

        let underline = if response.has_focus() || response.highlighted() {
            Stroke::new(1.0, response_color)
        } else {
            Stroke::NONE
        };

        let normal_label = || {
            ui.painter().add(epaint::TextShape {
                pos,
                galley: galley.clone(),
                underline,
                angle: 0.0,
                fallback_color: response_color,
                override_text_color: Some(response_color),
            });
        };
        if self.kinesis.is_none() {
            normal_label();
            return response;
        }

        let kes = self.kinesis.unwrap();
        let text_shape: TextShape = TextShape {
            pos,
            galley: galley.clone(),
            underline,
            angle: 0.0,
            fallback_color: response_color,
            override_text_color: Some(response_color),
        };
        let clipped_shape: ClippedShape = ClippedShape {
            clip_rect: Rect::EVERYTHING,
            shape: Shape::Text(text_shape),
        };
        let mut clipped_primitive = ui
            .ctx()
            .tessellate(vec![clipped_shape], galley.pixels_per_point);
        if clipped_primitive.is_empty() {
            normal_label();
            return response;
        };
        let Primitive::Mesh(ref mut the_mesh) = &mut clipped_primitive[0].primitive else {
            return response;
        };
        let len_vertices = the_mesh.vertices.len();
        kes.iter().for_each(|ke| match ke {
            KineticEffect::SineWavify { params } => {
                assert_ne!(params.live_dampen, 0.0);
                let mut vertical_translation = 0.;
                let framo = if params.live { ui.ctx().frame_nr() } else { 0 };
                for (i, v) in the_mesh.vertices.iter_mut().enumerate() {
                    let base_sinus_argument =
                        lerp(params.x_0..=params.x_1, i as f32 / len_vertices as f32);
                    // glyph quad border
                    if i % 4 == 0 {
                        vertical_translation =
                            (base_sinus_argument + (framo as f32) / params.live_dampen).sin()
                                * params.amp;
                    }
                    v.pos.y += vertical_translation;
                }
            }
            KineticEffect::ShakeLetters { params } => {
                let mut vertical_translation = 0;
                let mut horizontal_translation = 0;
                let mut rng = nanorand::tls_rng();
                for (i, v) in the_mesh.vertices.iter_mut().enumerate() {
                    // glyph quad border
                    if i % 4 == 0 && (ui.ctx().frame_nr() % params.dampen) < 5 {
                        vertical_translation = rng.generate_range(0..params.max_distortion);
                        horizontal_translation = rng.generate_range(0..params.max_distortion); // can't use -max..+max because then it averages out to the normal position lol
                        if rng.generate_range(0..100) > 50 {
                            vertical_translation = -vertical_translation;
                        } else {
                            horizontal_translation = -horizontal_translation;
                        }
                    }

                    v.pos.y += vertical_translation as f32;
                    v.pos.x += horizontal_translation as f32;
                }
            }
            KineticEffect::Gay { params } => {
                let mut colour = Color32::WHITE;
                let mut rainbow = params.rainbow.iter().cycle();
                for (i, v) in the_mesh.vertices.iter_mut().enumerate() {
                    // glyph quad border
                    if i % 4 == 0 {
                        if params.live {
                            for _ in 0..((ui.ctx().frame_nr() / params.live_dampen) % 8) + 1 {
                                rainbow.next();
                            }
                        }
                        colour = *rainbow.next().unwrap();
                    }
                    v.color = colour;
                }
            }
        });
        ui.painter().add(the_mesh.to_owned());
        response
    }
}
