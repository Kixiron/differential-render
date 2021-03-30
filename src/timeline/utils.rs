use crate::timeline::{
    constants::{MARGIN, X_LINE, Y_LINE, Y_TICK_DIST},
    properties::TimelineProps,
    RequiredLines,
};
use std::f64::consts::{FRAC_PI_2, PI};
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use yew::utils::document;

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

    let scale = properties.scale.ceil();
    let duration = properties.duration.ceil();

    let client_width = document().body().unwrap().client_width();

    // Cap the size of the graph. It is hard to view if it is too large, and
    // browsers may not render a large graph because it takes too much memory.
    // 4096 is still ridiculously large, and probably won't render on mobile
    // browsers, but should be ok for many desktop environments.
    // let graph_width = (scale * duration).min(4096.0);
    let graph_width = (client_width as f64 - X_LINE - 40.0).floor();
    let graph_height = Y_TICK_DIST * required_lines.required_lines() as f64; // Y_TICK_DIST * properties.events.len() as f64;

    let px_per_sec = (graph_width / duration).ceil();

    // let canvas_width = (graph_width + X_LINE + 30.0).max(X_LINE + 250.0);
    let canvas_width = (client_width as f64 - 10.0).max(X_LINE + 250.0);
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
