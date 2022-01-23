use piet::{
    kurbo::Line, kurbo::Point, kurbo::Rect, Color, Error, FontFamily, RenderContext, Text,
    TextAttribute, TextLayoutBuilder,
};

use osm2lanes::{Lane, LaneDirection, LanePrintable};

pub fn lanes<R: RenderContext>(
    rc: &mut R,
    (canvas_width, canvas_height): (u32, u32),
    lanes: &[Lane],
) -> Result<(), Error> {
    rc.clear(None, Color::OLIVE);

    let _canvas_width = canvas_width as f64;
    let canvas_height = canvas_height as f64;
    let grassy_verge = 10.0;
    let asphalt_buffer = 10.0;
    let lane_width = 80.0;
    rc.fill(
        Rect::new(
            grassy_verge,
            0.0,
            (grassy_verge + asphalt_buffer) + (lanes.len() as f64 * lane_width) + asphalt_buffer,
            canvas_height,
        ),
        &Color::BLACK,
    );
    let x = grassy_verge + asphalt_buffer - 5.0;
    rc.stroke(
        Line::new(
            Point { x, y: 0.0 },
            Point {
                x,
                y: canvas_height as f64,
            },
        ),
        &Color::WHITE,
        1.0,
    );
    let x = (grassy_verge + asphalt_buffer) + (lanes.len() as f64 * lane_width) + 5.0;
    rc.stroke(
        Line::new(
            Point { x, y: 0.0 },
            Point {
                x,
                y: canvas_height,
            },
        ),
        &Color::WHITE,
        1.0,
    );
    for (idx, lane) in lanes.iter().enumerate() {
        // asphalt
        rc.fill(
            Rect::new(
                (grassy_verge + asphalt_buffer) + (idx as f64 * lane_width),
                0.0,
                (grassy_verge + asphalt_buffer) + ((idx + 1) as f64 * lane_width),
                canvas_height,
            ),
            &Color::BLACK,
        );
        // left line
        let x = (grassy_verge + asphalt_buffer) + (idx as f64 * lane_width);
        rc.stroke(
            Line::new(
                Point { x, y: 0.0 },
                Point {
                    x,
                    y: canvas_height,
                },
            ),
            &Color::WHITE,
            1.0,
        );
        // right line
        let x = (grassy_verge + asphalt_buffer + lane_width) + (idx as f64 * lane_width);
        rc.stroke(
            Line::new(
                Point { x, y: 0.0 },
                Point {
                    x,
                    y: canvas_height,
                },
            ),
            &Color::WHITE,
            1.0,
        );
        // lane markings
        if let Lane::Travel {
            direction: Some(direction),
            ..
        } = lane
        {
            let x =
                (grassy_verge + asphalt_buffer) + (idx as f64 * lane_width) + (0.5 * lane_width);
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
        let layout = rc
            .text()
            .new_text_layout(lane.as_utf8().to_string())
            .font(FontFamily::SYSTEM_UI, 24.0)
            .default_attribute(TextAttribute::TextColor(Color::WHITE))
            .build()?;
        rc.draw_text(&layout, (x - 12.0, 0.5 * canvas_height));
    }

    rc.finish().unwrap();
    Ok(())
}

pub fn draw_arrow<R: RenderContext>(
    rc: &mut R,
    mid: Point,
    direction: LaneDirection,
) -> Result<(), Error> {
    fn draw_point<R: RenderContext>(
        rc: &mut R,
        mid: Point,
        direction: LaneDirection,
    ) -> Result<(), Error> {
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
