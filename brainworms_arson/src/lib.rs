#![feature(
    variant_count,
    exact_size_is_empty,
    array_chunks,
    iter_array_chunks,
    const_trait_impl,
    effects
)]

pub use anyhow;

pub use egui;
use egui::{
    egui_assert,
    epaint::{self, ClippedShape, Primitive, TextShape},
    lerp, pos2,
    text::LayoutJob,
    vec2, Align, Color32, Direction, FontFamily, FontId, FontSelection, Galley, Pos2, Rect,
    Response, Sense, Shape, Stroke, TextFormat, TextStyle, Ui, Widget, WidgetInfo, WidgetText,
    WidgetType,
};
pub use egui_wgpu;
pub use egui_winit;
pub use nanorand;
use nanorand::{RandomGen, Rng};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_till1},
    character::complete::{alpha1, anychar},
    combinator::{eof, map_opt, opt},
    error::{Error, ParseError},
    multi::{many1, separated_list1},
    number::{self, complete::be_u8},
    sequence::preceded,
    IResult, Parser,
};
use std::{
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    mem::variant_count,
    sync::Arc,
};
#[derive(Clone)]
pub struct KineticLabel {
    pub text: WidgetText,
    pub wrap: Option<bool>,
    pub truncate: bool,
    pub sense: Option<Sense>,
    pub kinesis: Option<Vec<KineticEffect>>,
}
#[derive(Clone, PartialEq, Debug)]
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
#[derive(Clone, PartialEq, Debug)]
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
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ShakeLetters {
    pub max_distortion: i32,
    pub dampen: u64,
}
#[derive(Copy, Clone, PartialEq, Debug)]
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

impl KineticLabel {
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
    pub fn kinesis(mut self, kinesis: Vec<KineticEffect>) -> Self {
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

impl KineticLabel {
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

impl Widget for KineticLabel {
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
            override_text_color: None, //            override_text_color: Some(response_color),
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
pub fn parse_fireworks(input: &str) -> IResult<&str, Vec<KineticLabel>> {
    let mut state = VecDeque::<TextModifier>::new();
    let mut out = Vec::new();
    let mut l_input = input;
    loop {
        let mut maybe_eof;
        (l_input, maybe_eof) = opt(eof).parse(l_input)?;
        if maybe_eof.is_some() {
            break;
        }
        let mut job = LayoutJob::default();

        let mut kinesis: Vec<KineticEffect> = Vec::new();
        let maybe_opening_transition: Option<Transition>;
        let body: &str;
        (l_input, maybe_opening_transition) = opt(parse_my_tag).parse(l_input)?;

        match maybe_opening_transition {
            Some(transition) => match transition {
                Transition::Enable(mfer) => match mfer {
                    TextModifier::PrevOpen => {
                        println!("Nonsense {{}} tag somehow made it into processing")
                    }
                    TextModifier::BuiltinOption(ref the_builtin) => match the_builtin {
                        BuiltinOption::FirstRowIndentation(_) => state.push_back(mfer),
                        BuiltinOption::Style(ref the_style) => match the_style {
                            TextStyle::Small => state.push_back(mfer),
                            TextStyle::Body => todo!(),
                            TextStyle::Monospace => state.push_back(mfer),
                            TextStyle::Button => todo!(),
                            TextStyle::Heading => state.push_back(mfer),
                            TextStyle::Name(_) => todo!(),
                        },
                        BuiltinOption::TextColor(_) => state.push_back(mfer),
                        BuiltinOption::BgColor(_) => todo!(),
                        BuiltinOption::FontStyle(_) => todo!(),
                        BuiltinOption::VerticalAlign(_) => todo!(),
                        BuiltinOption::Underline(_) => todo!(),
                        BuiltinOption::Strikethrough(_) => todo!(),
                        BuiltinOption::Italics => state.push_back(mfer),
                    },
                    TextModifier::KineticEffect(_) => state.push_back(mfer),

                    TextModifier::Unknown(_) => {
                        println!("Adding unknown {:?}", mfer);
                        state.push_back(mfer);
                    }
                },
                Transition::Disable(mfer) => match mfer {
                    TextModifier::PrevOpen => {
                        println!(
                            "removed {:?} from state by implicit {{/}} tag",
                            state.pop_back()
                        );
                    }
                    TextModifier::BuiltinOption(ref the_builtin) => match the_builtin {
                        BuiltinOption::FirstRowIndentation(_) => {
                            state
                                .remove(
                                    state
                                        .iter()
                                        .position(|tm| {
                                            matches!(
                                                tm,
                                                TextModifier::BuiltinOption(
                                                    BuiltinOption::FirstRowIndentation(_),
                                                )
                                            )
                                        })
                                        .unwrap(),
                                )
                                .unwrap();
                        }
                        BuiltinOption::Style(ref the_style) => match the_style {
                            TextStyle::Small => {
                                state
                                    .remove(state.iter().position(|tm| *tm == mfer).unwrap())
                                    .unwrap();
                            }
                            TextStyle::Body => todo!(), // annoying to implement and easy to work around
                            TextStyle::Monospace => {
                                state
                                    .remove(state.iter().position(|tm| *tm == mfer).unwrap())
                                    .unwrap();
                            }
                            TextStyle::Button => todo!(), // don't care
                            TextStyle::Heading => {
                                state
                                    .remove(state.iter().position(|tm| *tm == mfer).unwrap())
                                    .unwrap();
                            }
                            TextStyle::Name(_) => todo!(), // maybe later
                        },
                        BuiltinOption::TextColor(_) => {
                            state
                                .remove(
                                    state
                                        .iter()
                                        .position(|tm| {
                                            matches!(
                                                tm,
                                                TextModifier::BuiltinOption(
                                                    BuiltinOption::TextColor(_)
                                                )
                                            )
                                        })
                                        .unwrap(),
                                )
                                .unwrap();
                        }
                        BuiltinOption::BgColor(_) => todo!(),
                        BuiltinOption::FontStyle(_) => todo!(),
                        BuiltinOption::VerticalAlign(_) => todo!(),
                        BuiltinOption::Underline(_) => todo!(),
                        BuiltinOption::Strikethrough(_) => todo!(),
                        BuiltinOption::Italics => {
                            state
                                .remove(state.iter().position(|tm| *tm == mfer).unwrap())
                                .unwrap();
                        }
                    },
                    // this can probably be simplified to just compare the modifiers
                    TextModifier::KineticEffect(the_effect) => {
                        state.remove(
                            state
                                .iter()
                                .position(|tm| {
                                    if let TextModifier::KineticEffect(an_effect) = tm {
                                        *an_effect == the_effect
                                    } else {
                                        false
                                    }
                                })
                                .unwrap(),
                        );
                    }
                    TextModifier::Unknown((name, _)) => {
                        state.remove(
                            state
                                .iter()
                                .position(|tm| {
                                    if let TextModifier::Unknown((other_name, _)) = tm {
                                        *other_name == name
                                    } else {
                                        false
                                    }
                                })
                                .unwrap(),
                        );
                    }
                },
            },
            None => {
                (l_input, maybe_eof) = opt(eof).parse(l_input)?;
                if maybe_eof.is_some() {
                    break;
                }
                (l_input, body) = take_till1(|c| c == '{').parse(l_input)?;
                job.append(body, 0., TextFormat::default());
                let lay_section = job.sections.first_mut().unwrap();
                for mfer in state.iter() {
                    match mfer {
                        TextModifier::PrevOpen => {}
                        TextModifier::BuiltinOption(ref the_builtin) => match the_builtin {
                            BuiltinOption::FirstRowIndentation(length) => {
                                lay_section.leading_space = *length
                            }
                            BuiltinOption::Style(ref the_style) => match the_style {
                                TextStyle::Small => {
                                    let FontId { size, family } =
                                        lay_section.format.font_id.clone();

                                    lay_section.format.font_id = FontId {
                                        size: size * 0.5,
                                        family,
                                    }
                                }

                                TextStyle::Body => todo!(), // not implementing this
                                TextStyle::Monospace => {
                                    lay_section.format.font_id.family = FontFamily::Monospace
                                }
                                TextStyle::Button => todo!(), // don't care
                                TextStyle::Heading => lay_section.format.font_id.size *= 2.0,
                                TextStyle::Name(_) => todo!(), // don't care for now, maybe later
                            },
                            BuiltinOption::TextColor(the_color) => {
                                lay_section.format.color = the_color.to_owned()
                            }
                            BuiltinOption::BgColor(_) => todo!(),
                            BuiltinOption::FontStyle(_) => todo!(),
                            BuiltinOption::VerticalAlign(_) => todo!(),
                            BuiltinOption::Underline(_) => todo!(),
                            BuiltinOption::Strikethrough(_) => todo!(),
                            BuiltinOption::Italics => lay_section.format.italics = true,
                        },
                        TextModifier::KineticEffect(the_effect) => kinesis.push(the_effect.clone()),
                        TextModifier::Unknown((um, uma)) => {
                            println!("Unknown modifier {um} with args {uma}");
                        }
                    }
                }
                out.push(KineticLabel::new(job).kinesis(kinesis));
            }
        }
        //        out.push(job);
    }
    //    let out = KineticLabel::new(job);

    Ok((input, out))
}
//pub fn parse_egui_builtin(input: &str) {}
pub fn parse_my_tag(input: &str) -> IResult<&str, Transition> {
    let (input, _) = tag("{").parse(input)?;
    let (input, close_my_tag) = opt(tag("/"))
        .parse(input)
        .map(|(input, cmt)| (input, cmt.is_some()))?;
    let (input, the_modifier) = parse_text_modifier(input)?;
    let (input, _) = tag("}").parse(input)?;
    if close_my_tag {
        Ok((input, Transition::Disable(the_modifier)))
    } else {
        Ok((input, Transition::Enable(the_modifier)))
    }
}
fn parse_color(input: &str) -> IResult<&str, Color32> {
    const KNOWN_COLORS: [(&str, Color32); 20] = [
        ("TRANSPARENT", Color32::TRANSPARENT),
        ("BLACK", Color32::BLACK),
        ("DARK_GRAY", Color32::DARK_GRAY),
        ("GRAY", Color32::GRAY),
        ("LIGHT_GRAY", Color32::LIGHT_GRAY),
        ("WHITE", Color32::WHITE),
        ("BROWN", Color32::BROWN),
        ("DARK_RED", Color32::DARK_RED),
        ("RED", Color32::RED),
        ("LIGHT_RED", Color32::LIGHT_RED),
        ("YELLOW", Color32::YELLOW),
        ("LIGHT_YELLOW", Color32::LIGHT_YELLOW),
        ("KHAKI", Color32::KHAKI),
        ("DARK_GREEN", Color32::DARK_GREEN),
        ("GREEN", Color32::GREEN),
        ("LIGHT_GREEN", Color32::LIGHT_GREEN),
        ("DARK_BLUE", Color32::DARK_BLUE),
        ("BLUE", Color32::BLUE),
        ("LIGHT_BLUE", Color32::LIGHT_BLUE),
        ("GOLD", Color32::GOLD),
    ];
    let known_colors = HashMap::from(KNOWN_COLORS);
    let ret = map_opt(
        preceded(
            tag("rgb"),
            nom::sequence::delimited(
                tag::<&str, &str, Error<_>>("("),
                separated_list1(tag(","), nom::character::complete::u8),
                tag(")"),
            ),
        ),
        |kulerz| Some(Color32::from_rgb(kulerz[0], kulerz[1], kulerz[2])),
    )
    .parse(input)
    .or(map_opt(many1(anychar), |wc: Vec<char>| {
        known_colors
            .get(wc.iter().collect::<String>().to_uppercase().as_str())
            .map(|v| v.to_owned())
    })
    .parse(input));
    ret
}
fn parse_text_modifier(input: &str) -> IResult<&str, TextModifier> {
    let (input, full_modifier) = take_till(|c| c == '}').parse(input)?;
    let (rest, modifier_name) = opt(many1(alt((tag("_"), alpha1))))
        .parse(full_modifier)
        .map(|(r, frags)| (r, frags.map(|frags| frags.join(""))))?;
    match modifier_name {
        Some(modifier_name) => {
            let modifier_args = opt(tag("="))
                .parse(rest)
                .map(|(modifier_args, _)| modifier_args)?;
            match modifier_name.as_str() {
                "color" => {
                    let the_color = match parse_color(modifier_args) {
                        Ok((_, c)) => c,
                        Err(e) => {
                            println!("{:?}", e);
                            Color32::default()
                        }
                    };
                    println!("adding text color {:?}", the_color);
                    Ok((
                        input,
                        TextModifier::BuiltinOption(BuiltinOption::TextColor(the_color)),
                    ))
                }
                "h" => Ok((
                    input,
                    TextModifier::BuiltinOption(BuiltinOption::Style(TextStyle::Heading)),
                )),
                "mono" => Ok((
                    input,
                    TextModifier::BuiltinOption(BuiltinOption::Style(TextStyle::Monospace)),
                )),
                "p" => Ok((
                    input,
                    TextModifier::BuiltinOption(BuiltinOption::FirstRowIndentation(10.0)),
                )),
                "gay" => Ok((
                    input,
                    TextModifier::KineticEffect(KineticEffect::Gay {
                        params: Gay::default(),
                    }),
                )),
                "shakey" => Ok((
                    input,
                    TextModifier::KineticEffect(KineticEffect::ShakeLetters {
                        params: ShakeLetters::default(),
                    }),
                )),
                "wavy" => Ok((
                    input,
                    TextModifier::KineticEffect(KineticEffect::SineWavify {
                        params: SineWavify::default(),
                    }),
                )),

                "i" => Ok((input, TextModifier::BuiltinOption(BuiltinOption::Italics))),
                "small" => Ok((
                    input,
                    TextModifier::BuiltinOption(BuiltinOption::Style(TextStyle::Small)),
                )),
                blah => Ok((
                    input,
                    TextModifier::Unknown((blah.to_owned(), modifier_args.to_owned())),
                )),
            }
        }

        None => Ok((input, TextModifier::PrevOpen)),
    }
}
#[derive(Debug, PartialEq)]
pub enum Transition {
    Enable(TextModifier),
    Disable(TextModifier),
}
#[derive(Debug, PartialEq, Default)]
pub enum TextModifier {
    #[default]
    PrevOpen,
    BuiltinOption(BuiltinOption),
    KineticEffect(KineticEffect),
    Unknown((String, String)),
}
#[derive(Debug, PartialEq)]
pub enum BuiltinOption {
    FirstRowIndentation(f32),
    Style(TextStyle),
    TextColor(Color32),
    BgColor(Color32),
    FontStyle(FontId),

    VerticalAlign(Align),
    Underline(Stroke),
    Strikethrough(Stroke),
    //    StrongText, this isn't really implemented in egui
    Italics,
}
