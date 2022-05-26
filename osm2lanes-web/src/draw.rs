use osm2lanes::locale::Locale;
use osm2lanes::metric::Metre;
use osm2lanes::road::{Color as MarkingColor, Designated, Direction, Lane, Printable, Road, Style};
use piet::kurbo::{Line, Point, Rect};
use piet::{
    Color as PietColor, FontFamily, RenderContext, StrokeStyle, Text, TextAttribute,
    TextLayoutBuilder,
};

use crate::canvas::RenderError;

// TODO: newtype + From?
fn color_into(c: MarkingColor) -> PietColor {
    match c {
        MarkingColor::White => PietColor::WHITE,
        MarkingColor::Yellow => PietColor::YELLOW,
        MarkingColor::Red => PietColor::RED,
        MarkingColor::Green => PietColor::GREEN,
    }
}

struct Scale(f64);

impl Scale {
    fn scale(&self, m: Metre) -> f64 {
        self.0 * m.val()
    }
}

pub(crate) fn lanes<R: RenderContext>(
    rc: &mut R,
    (canvas_width, canvas_height): (u32, u32),
    road: &Road,
    locale: &Locale,
) -> Result<(), RenderError> {
    let canvas_width = f64::from(canvas_width);
    let canvas_height = f64::from(canvas_height);
    let default_lane_width = Lane::DEFAULT_WIDTH;

    let grassy_verge = Metre::new(1.0);
    let asphalt_buffer = Metre::new(0.1);

    let scale = Scale(
        canvas_width / (road.width(locale) + 2.0 * grassy_verge + 2.0 * asphalt_buffer).val(),
    );

    // Background
    rc.clear(None, PietColor::OLIVE);

    rc.fill(
        Rect::new(
            scale.scale(grassy_verge),
            0.0,
            scale.scale(grassy_verge + asphalt_buffer + road.width(locale) + asphalt_buffer),
            canvas_height,
        ),
        &PietColor::BLACK,
    );

    let mut left_edge = grassy_verge + asphalt_buffer;

    for lane in &road.lanes {
        match lane {
            Lane::Travel {
                direction,
                designated,
                width,
                ..
            } => draw_travel_lane(
                rc,
                &mut left_edge,
                road,
                locale,
                *designated,
                width,
                &scale,
                *direction,
                canvas_height,
                lane,
            )?,
            Lane::Parking {
                designated, width, ..
            } => {
                let width =
                    width.unwrap_or_else(|| locale.travel_width(designated, road.highway.r#type()));
                let x = scale.scale(left_edge + (0.5 * width));
                let font_size = 24.0;
                let layout = rc
                    .text()
                    .new_text_layout(lane.as_utf8().to_string())
                    .font(FontFamily::SYSTEM_UI, font_size)
                    .default_attribute(TextAttribute::TextColor(PietColor::WHITE))
                    .build()?;
                rc.draw_text(&layout, (x - (0.5 * font_size), 0.5 * canvas_height));
                left_edge += width;
            },
            Lane::Shoulder { width } => {
                let width = width.unwrap_or(default_lane_width);
                let x = scale.scale(left_edge + (0.5 * width));
                let font_size = 24.0;
                let layout = rc
                    .text()
                    .new_text_layout(lane.as_utf8().to_string())
                    .font(FontFamily::SYSTEM_UI, font_size)
                    .default_attribute(TextAttribute::TextColor(PietColor::WHITE))
                    .build()?;
                rc.draw_text(&layout, (x - (0.5 * font_size), 0.5 * canvas_height));
                left_edge += width;
            },
            Lane::Separator { markings, .. } => {
                draw_separator(rc, &mut left_edge, markings, &scale, canvas_height);
            },
        }
    }

    rc.finish().unwrap();
    Ok(())
}

// TODO: refactor
#[allow(clippy::too_many_arguments)]
fn draw_travel_lane<R: RenderContext>(
    rc: &mut R,
    left_edge: &mut Metre,
    road: &Road,
    locale: &Locale,
    designated: Designated,
    width: &Option<Metre>,
    scale: &Scale,
    direction: Option<Direction>,
    canvas_height: f64,
    lane: &Lane,
) -> Result<(), RenderError> {
    let width = width.unwrap_or_else(|| locale.travel_width(&designated, road.highway.r#type()));
    let x = scale.scale(*left_edge + (0.5 * width));
    if let Some(direction) = direction {
        draw_arrow(
            rc,
            Point {
                x,
                y: 0.3 * canvas_height,
            },
            direction,
        );
        draw_arrow(
            rc,
            Point {
                x,
                y: 0.7 * canvas_height,
            },
            direction,
        );
    }
    if lane.is_foot() {
        rc.fill(
            Rect::new(
                scale.scale(*left_edge),
                0.0,
                scale.scale(*left_edge + width),
                canvas_height,
            ),
            &PietColor::GRAY,
        );
    }
    let font_size = 24.0;
    let layout = rc
        .text()
        .new_text_layout(lane.as_utf8().to_string())
        .font(FontFamily::SYSTEM_UI, font_size)
        .default_attribute(TextAttribute::TextColor(PietColor::WHITE))
        .build()?;
    rc.draw_text(&layout, (x - (0.5 * font_size), 0.5 * canvas_height));
    *left_edge += width;
    Ok(())
}

fn draw_separator<R: RenderContext>(
    rc: &mut R,
    left_edge: &mut Metre,
    markings: &osm2lanes::road::Markings,
    scale: &Scale,
    canvas_height: f64,
) {
    for marking in markings.iter() {
        let width = marking.width.unwrap_or_else(|| Metre::new(0.2));
        let x = scale.scale(*left_edge + 0.5 * width);
        let color = match (marking.style, marking.color) {
            (_, Some(c)) => color_into(c),
            (Style::KerbUp | Style::KerbDown, None) => PietColor::GRAY,
            // Remains for debugging
            _ => PietColor::BLUE,
            // _ => return Err(RenderError::UnknownSeparator),
        };
        if let Style::NoFill = marking.style {
            // nope
        } else {
            rc.stroke_styled(
                Line::new(
                    Point { x, y: 0.0 },
                    Point {
                        x,
                        y: canvas_height,
                    },
                ),
                &color,
                scale.scale(width),
                &match marking.style {
                    Style::DottedLine => StrokeStyle::new().dash_pattern(&[50.0, 100.0]),
                    Style::DashedLine => StrokeStyle::new().dash_pattern(&[100.0, 100.0]),
                    Style::BrokenLine => StrokeStyle::new().dash_pattern(&[100.0, 50.0]),
                    Style::SolidLine | Style::KerbUp | Style::KerbDown => StrokeStyle::new(),
                    Style::NoFill => unreachable!(),
                    // _ => return Err(RenderError::UnknownSeparator),
                },
            );
        }
        *left_edge += width;
    }
}

fn draw_arrow<R: RenderContext>(rc: &mut R, mid: Point, direction: Direction) {
    fn draw_point<R: RenderContext>(rc: &mut R, mid: Point, direction: Direction) {
        let dir_sign = match direction {
            Direction::Forward => -1.0,
            Direction::Backward => 1.0,
            Direction::Both => unreachable!(),
        };
        for x in [-10.0, 10.0] {
            rc.stroke(
                Line::new(
                    Point {
                        x: mid.x,
                        y: mid.y + dir_sign * 20.0,
                    },
                    Point {
                        x: mid.x + x,
                        y: mid.y + dir_sign * 10.0,
                    },
                ),
                &PietColor::WHITE,
                1.0,
            );
        }
    }
    // line
    rc.stroke(
        Line::new(
            Point {
                x: mid.x,
                y: mid.y - 20.0,
            },
            Point {
                x: mid.x,
                y: mid.y + 20.0,
            },
        ),
        &PietColor::WHITE,
        1.0,
    );
    match direction {
        Direction::Forward | Direction::Backward => draw_point(rc, mid, direction),
        Direction::Both => {
            draw_point(rc, mid, Direction::Forward);
            draw_point(rc, mid, Direction::Backward);
        },
    }
}
