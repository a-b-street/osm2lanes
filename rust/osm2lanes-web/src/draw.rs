use osm2lanes::road::{Lane, LaneDirection, LanePrintable, MarkingColor, MarkingStyle, Road};
use osm2lanes::{Locale, Metre};
use piet::kurbo::{Line, Point, Rect};
use piet::{Color, FontFamily, RenderContext, StrokeStyle, Text, TextAttribute, TextLayoutBuilder};

use super::RenderError;

// TODO: newtype + From?
fn color_into(c: MarkingColor) -> Color {
    match c {
        MarkingColor::White => Color::WHITE,
        MarkingColor::Yellow => Color::YELLOW,
        MarkingColor::Red => Color::RED,
    }
}

struct Scale(f64);

impl Scale {
    fn scale(&self, m: Metre) -> f64 {
        self.0 * m.val()
    }
}

pub fn lanes<R: RenderContext>(
    rc: &mut R,
    (canvas_width, canvas_height): (u32, u32),
    road: &Road,
    locale: &Locale,
) -> Result<(), RenderError> {
    let canvas_width = canvas_width as f64;
    let canvas_height = canvas_height as f64;
    let default_lane_width = Lane::DEFAULT_WIDTH;

    let grassy_verge = Metre::new(1.0);
    let asphalt_buffer = Metre::new(0.1);

    let scale = Scale(
        canvas_width / (road.width(locale) + 2.0 * grassy_verge + 2.0 * asphalt_buffer).val(),
    );

    // Background
    rc.clear(None, Color::OLIVE);

    rc.fill(
        Rect::new(
            scale.scale(grassy_verge),
            0.0,
            scale.scale(grassy_verge + asphalt_buffer + road.width(locale) + asphalt_buffer),
            canvas_height,
        ),
        &Color::BLACK,
    );

    let mut left_edge = grassy_verge + asphalt_buffer;

    for lane in &road.lanes {
        match lane {
            Lane::Travel {
                direction,
                designated,
            } => {
                let width = locale.default_width(designated);
                let x = scale.scale(left_edge + (0.5 * width));
                if let Some(direction) = direction {
                    draw_arrow(
                        rc,
                        Point {
                            x,
                            y: 0.3 * canvas_height,
                        },
                        *direction,
                    )?;
                    draw_arrow(
                        rc,
                        Point {
                            x,
                            y: 0.7 * canvas_height,
                        },
                        *direction,
                    )?;
                }
                if lane.is_foot() {
                    rc.fill(
                        Rect::new(
                            scale.scale(left_edge),
                            0.0,
                            scale.scale(left_edge + width),
                            canvas_height,
                        ),
                        &Color::GRAY,
                    );
                }
                let font_size = 24.0;
                let layout = rc
                    .text()
                    .new_text_layout(lane.as_utf8().to_string())
                    .font(FontFamily::SYSTEM_UI, font_size)
                    .default_attribute(TextAttribute::TextColor(Color::WHITE))
                    .build()?;
                rc.draw_text(&layout, (x - (0.5 * font_size), 0.5 * canvas_height));
                left_edge += width;
            }
            Lane::Shoulder => {
                let width = default_lane_width;
                let x = scale.scale(left_edge + (0.5 * width));
                let font_size = 24.0;
                let layout = rc
                    .text()
                    .new_text_layout(lane.as_utf8().to_string())
                    .font(FontFamily::SYSTEM_UI, font_size)
                    .default_attribute(TextAttribute::TextColor(Color::WHITE))
                    .build()?;
                rc.draw_text(&layout, (x - (0.5 * font_size), 0.5 * canvas_height));
                left_edge += width;
            }
            Lane::Separator { markings } => {
                for marking in markings.iter() {
                    let width = marking.width.unwrap_or_else(|| Metre::new(0.2));
                    let x = scale.scale(left_edge + 0.5 * width);
                    let color = match (marking.style, marking.color) {
                        (_, Some(c)) => color_into(c),
                        (MarkingStyle::KerbUp | MarkingStyle::KerbDown, None) => Color::GRAY,
                        // Remains for debugging
                        _ => Color::BLUE,
                        // _ => return Err(RenderError::UnknownSeparator),
                    };
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
                            MarkingStyle::SolidLine => StrokeStyle::new(),
                            MarkingStyle::DottedLine => {
                                StrokeStyle::new().dash_pattern(&[50.0, 100.0])
                            }
                            MarkingStyle::KerbUp | MarkingStyle::KerbDown => StrokeStyle::new(),
                            // Remains for debugging
                            _ => StrokeStyle::new().dash_pattern(&[20.0, 80.0]),
                            // _ => return Err(RenderError::UnknownSeparator),
                        },
                    );
                    left_edge += width;
                }
            }
            _ => return Err(RenderError::UnknownLane),
        }
    }

    rc.finish().unwrap();
    Ok(())
}

pub fn draw_arrow<R: RenderContext>(
    rc: &mut R,
    mid: Point,
    direction: LaneDirection,
) -> Result<(), RenderError> {
    fn draw_point<R: RenderContext>(
        rc: &mut R,
        mid: Point,
        direction: LaneDirection,
    ) -> Result<(), RenderError> {
        let dir_sign = match direction {
            LaneDirection::Forward => -1.0,
            LaneDirection::Backward => 1.0,
            _ => unreachable!(),
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
                &Color::WHITE,
                1.0,
            );
        }
        Ok(())
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
        &Color::WHITE,
        1.0,
    );
    match direction {
        LaneDirection::Forward | LaneDirection::Backward => draw_point(rc, mid, direction)?,
        LaneDirection::Both => {
            draw_point(rc, mid, LaneDirection::Forward)?;
            draw_point(rc, mid, LaneDirection::Backward)?;
        }
    }
    Ok(())
}
