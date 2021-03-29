use crate::timeline::{
    constants::{MARGIN, MIN_TICK_DIST, X_LINE, Y_LINE, Y_TICK_DIST},
    properties::TimelineProps,
    RequiredLines,
};
use std::f64::consts::{FRAC_PI_2, PI};
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

// Returns `(step, tick_distance, number_ticks)`
pub(super) fn split_ticks(max_value: f64, px_per_val: f64, max_px: f64) -> (f64, f64, usize) {
    let max_ticks = (max_px / MIN_TICK_DIST).floor();

    // The graph is too small for even 1 tick
    if max_ticks <= 1.0 {
        tracing::debug!("graph too small, using 1 as the tick size");
        return (max_value, max_px, 1);
    }

    let step = if max_value <= max_ticks {
        1.0
    } else if max_value <= max_ticks * 2.0 {
        2.0
    } else if max_value <= max_ticks * 4.0 {
        4.0
    } else if max_value <= max_ticks * 5.0 {
        5.0
    } else {
        let mut step = 10.0;

        let mut count = 0;
        loop {
            if count > 100 {
                panic!("tick loop too long");
            }
            count += 1;

            if max_value <= max_ticks * step {
                break;
            }

            step += 10.0;
        }

        step
    };

    let tick_dist = px_per_val * step;
    let num_ticks = (max_value / step).floor() as usize;

    (step, tick_dist, num_ticks)
}

pub(super) fn draw_rounded_rect(
    ctx: &CanvasRenderingContext2d,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius: f64,
) -> Result<(), JsValue> {
    let radius = radius.min(width).min(height);

    ctx.begin_path();
    ctx.move_to(x + radius, y);

    ctx.line_to(x + width - radius, y);
    ctx.arc(x + width - radius, y + radius, radius, 3.0 * FRAC_PI_2, 0.0)?;

    ctx.line_to(x + width, y + height - radius);
    ctx.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        FRAC_PI_2,
    )?;

    ctx.line_to(x + radius, y + height);
    ctx.arc(x + radius, y + height - radius, radius, FRAC_PI_2, PI)?;

    ctx.line_to(x, y - radius);
    ctx.arc(x + radius, y + radius, radius, PI, 3.0 * FRAC_PI_2)?;

    ctx.close_path();
    Ok(())
}

pub(super) fn calculate_timeline_dimensions(
    properties: &TimelineProps,
) -> (f64, f64, f64, f64, f64, f64, f64, RequiredLines) {
    let required_lines = RequiredLines::new(&*properties.events);

    let scale = properties.scale;
    let duration = properties.duration.ceil();

    // Cap the size of the graph. It is hard to view if it is too large, and
    // browsers may not render a large graph because it takes too much memory.
    // 4096 is still ridiculously large, and probably won't render on mobile
    // browsers, but should be ok for many desktop environments.
    let graph_width = (scale * duration).min(4096.0);
    let graph_height = Y_TICK_DIST * required_lines.required_lines() as f64; // Y_TICK_DIST * properties.events.len() as f64;

    let px_per_sec = graph_width / duration;

    let canvas_width = (graph_width + X_LINE + 30.0).max(X_LINE + 250.0);
    let canvas_height = graph_height + MARGIN + Y_LINE;

    tracing::debug!(
        scale = %scale,
        duration = %duration,
        px_per_sec = %px_per_sec,
        graph_width = %graph_width,
        graph_height = %graph_height,
        canvas_width = %canvas_width,
        canvas_height = %canvas_height,
        required_lines = ?required_lines,
        "calculated timeline dimensions",
    );

    (
        scale,
        duration,
        graph_width,
        graph_height,
        px_per_sec,
        canvas_width,
        canvas_height,
        required_lines,
    )
}
