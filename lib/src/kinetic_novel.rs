use std::f32::consts::PI;

use egui::{
    egui_assert,
    epaint::{self, ClippedShape, Primitive, TextShape},
    lerp, pos2, vec2,
    widget_text::WidgetTextGalley,
    Align, Direction, FontSelection, FullOutput, Pos2, Rect, Response, Sense, Shape, Stroke, Ui,
    Widget, WidgetInfo, WidgetText, WidgetType,
};
use log::info;

pub(crate) struct KineticLabel {
    text: WidgetText,
    wrap: Option<bool>,
    truncate: bool,
    sense: Option<Sense>,
}

impl KineticLabel {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            wrap: None,
            truncate: false,
            sense: None,
        }
    }

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
    /// You can also use [`crate::Style::wrap`].
    #[inline]
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
    /// # use egui::{KineticLabel, Sense};
    /// # egui::__run_test_ui(|ui| {
    /// if ui.add(KineticLabel::new("click me").sense(Sense::click())).clicked() {
    ///     /* … */
    /// }
    /// # });
    /// ```
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }
}

impl KineticLabel {
    /// Do layout and position the galley in the ui, without painting it or adding widget info.
    pub fn layout_in_ui(self, ui: &mut Ui) -> (Pos2, WidgetTextGalley, Response) {
        let sense = self.sense.unwrap_or_else(|| {
            // We only want to focus labels if the screen reader is on.
            if ui.memory(|mem| mem.options.screen_reader) {
                Sense::focusable_noninteractive()
            } else {
                Sense::hover()
            }
        });
        if let WidgetText::Galley(galley) = self.text {
            // If the user said "use this specific galley", then just use it:
            let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
            let pos = match galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };
            let text_galley = WidgetTextGalley {
                galley,
                galley_has_color: true,
            };
            return (pos, text_galley, response);
        }

        let valign = ui.layout().vertical_align();
        let mut text_job = self
            .text
            .into_text_job(ui.style(), FontSelection::Default, valign);

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

            text_job.job.wrap.max_width = available_width;
            text_job.job.first_row_min_height = cursor.height();
            text_job.job.halign = Align::Min;
            text_job.job.justify = false;
            if let Some(first_section) = text_job.job.sections.first_mut() {
                first_section.leading_space = first_row_indentation;
            }
            let text_galley = ui.fonts(|f| text_job.into_galley(f));

            let pos = pos2(ui.max_rect().left(), ui.cursor().top());
            assert!(
                !text_galley.galley.rows.is_empty(),
                "Galleys are never empty"
            );
            // collect a response from many rows:
            let rect = text_galley.galley.rows[0]
                .rect
                .translate(vec2(pos.x, pos.y));
            let mut response = ui.allocate_rect(rect, sense);
            for row in text_galley.galley.rows.iter().skip(1) {
                let rect = row.rect.translate(vec2(pos.x, pos.y));
                response |= ui.allocate_rect(rect, sense);
            }
            (pos, text_galley, response)
        } else {
            if truncate {
                text_job.job.wrap.max_width = available_width;
                text_job.job.wrap.max_rows = 1;
                text_job.job.wrap.break_anywhere = true;
            } else if wrap {
                text_job.job.wrap.max_width = available_width;
            } else {
                text_job.job.wrap.max_width = f32::INFINITY;
            };
            /*
            // is_grid is private
            if ui.is_grid() {
                // TODO(emilk): remove special Grid hacks like these
                text_job.job.halign = Align::LEFT;
                text_job.job.justify = false;
            } else {
                text_job.job.halign = ui.layout().horizontal_placement();
                text_job.job.justify = ui.layout().horizontal_justify();
            };
            */
            text_job.job.halign = ui.layout().horizontal_placement();
            text_job.job.justify = ui.layout().horizontal_justify();
            let text_galley = ui.fonts(|f| text_job.into_galley(f));
            let (rect, response) = ui.allocate_exact_size(text_galley.size(), sense);
            let pos = match text_galley.galley.job.halign {
                Align::LEFT => rect.left_top(),
                Align::Center => rect.center_top(),
                Align::RIGHT => rect.right_top(),
            };
            (pos, text_galley, response)
        }
    }
}

impl Widget for KineticLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let (pos, text_galley, mut response) = self.layout_in_ui(ui);

        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, text_galley.text()));

        if text_galley.galley.elided {
            // Show the full (non-elided) text on hover:
            response = response.on_hover_text(text_galley.text());
        }

        if ui.is_rect_visible(response.rect) {
            let response_color = ui.style().interact(&response).text_color();

            let underline = if response.has_focus() || response.highlighted() {
                Stroke::new(1.0, response_color)
            } else {
                Stroke::NONE
            };

            let override_text_color = if text_galley.galley_has_color {
                None
            } else {
                Some(response_color)
            };
            let hmmshape: TextShape = TextShape {
                pos,
                galley: text_galley.galley.clone(),
                underline,
                override_text_color,
                angle: 0.0,
            };
            let humshape: ClippedShape = ClippedShape {
                clip_rect: Rect::EVERYTHING,
                shape: Shape::Text(hmmshape),
            };
            let homshape = ui
                .ctx()
                .tessellate(vec![humshape], text_galley.galley.pixels_per_point);

            let mymesh = homshape[0].primitive.clone();
            if let Primitive::Mesh(mut themesh) = mymesh {
                let lenne = themesh.vertices.len();

                for (i, v) in themesh.vertices.iter_mut().enumerate() {
                    let ok = lerp(-PI..=PI, i as f32 / lenne as f32);
                    //                    info!("{}", ok);
                    v.pos.y += ok * 1000.;
                }
                ui.painter().add(themesh);
            }

            /*
            ui.painter().add(epaint::TextShape {
                pos,
                galley: text_galley.galley,
                override_text_color,
                underline,
                angle: 0.0,
            });
            */
        }

        response
    }
}