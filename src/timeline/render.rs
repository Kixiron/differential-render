use crate::{
    data::TimelineEvent,
    timeline::{
        constants::{BOX_HEIGHT, MARGIN, RADIUS, X_LINE, Y_LINE, Y_TICK_DIST},
        utils::draw_rounded_rect,
        Hitbox, Timeline,
    },
};
use gigatrace::trace::Nanos;
use humantime::Duration as HumanDuration;
use std::{f64::consts::PI, ops::Range, time::Duration};
use web_sys::{CanvasRenderingContext2d, HtmlDivElement};

impl Timeline {
    pub(super) fn render_timeline(&mut self, ctx: &CanvasRenderingContext2d, timestamp: f64) {
        tracing::trace!(
            timestamp = %timestamp,
            num_events = self.events.len(),
            "started rendering timeline",
        );

        if self.events.is_empty() {
            tracing::trace!("timeline has no events, skipping rendering");
            return;
        }

        // ctx.save();
        // ctx.set_fill_style(&self.background_color);
        // ctx.fill_rect(0.0, 0.0, self.canvas_width, self.canvas_height);
        // ctx.restore();

        let view = ViewMap::new(&self.view_range, self.graph_width);
        let quant = ViewQuant::new(&self.view_range, self.graph_width);

        // Draw the graph's x axis
        self.draw_x_axis(ctx, &view, &quant);

        // Draw the graph's y axis
        let max_idx = self.required_lines.required_lines().saturating_sub(1);
        for (idx, event_kind) in self.required_lines.events().enumerate() {
            self.draw_y_line(&event_kind, idx, max_idx, ctx);
        }

        ctx.save();
        ctx.translate(X_LINE, MARGIN).unwrap();

        tracing::debug!(
            view_range = ?self.view_range,
            quantized_range = ?quant.quantize(&self.view_range),
            time_step = quant.time_step,
            time_bounds = ?self.trace.time_bounds(),
        );

        for (track_idx, track) in self.trace.tracks.iter().enumerate() {
            // TODO: Buffer this
            let visible_events = gigatrace::aggregate_by_steps(
                &self.trace.pool,
                &track.track.block_locations,
                &track.zoom_index,
                quant.quantize(&self.view_range),
                quant.time_step,
            );

            for event in visible_events.iter().filter_map(|event| event.0) {
                let timestamp = event.timestamp.unpack();
                let duration = event.duration.unpack();

                let (start, width) = if duration > quant.time_step {
                    (timestamp, duration)
                } else {
                    (quant.round_down(timestamp), quant.time_step)
                };
                let (start, width) = (view.to_x(start), view.to_x(width));

                let y = track_idx as f64 * Y_TICK_DIST + 1.0;
                let height = BOX_HEIGHT;

                ctx.begin_path();
                ctx.set_fill_style(&self.block_rect_fill);
                draw_rounded_rect(ctx, start, y, width, height, RADIUS).unwrap();
                ctx.fill();

                ctx.set_fill_style(&self.black);
                ctx.set_text_align("start");
                ctx.set_text_baseline("hanging");
                ctx.set_font("14px sans-serif");

                // TODO: Format buffer
                let label = format!(
                    "{}",
                    HumanDuration::from(Duration::from_nanos(event.duration.unpack())),
                );
                let text_info = ctx.measure_text(&label).unwrap();
                let label_x = (start + 5.0).min(self.canvas_width - text_info.width() - X_LINE);

                ctx.fill_text(&label, label_x, y + height / 2.0 - 5.0)
                    .unwrap();

                let timeline_event = self.event_map.get(&event.kind).unwrap();
                self.hitboxes.push(Hitbox {
                    x: start + X_LINE,
                    y: y + MARGIN,
                    width,
                    height,
                    tooltip: format!("{} ran for {}", &timeline_event.event, &label),
                });
            }
        }

        ctx.restore();

        tracing::trace!(
            timestamp = %timestamp,
            "finished rendering timeline",
        );
    }

    pub(super) fn render_overlay(&self, ctx: &CanvasRenderingContext2d, timestamp: f64) {
        tracing::trace!(
            timestamp = %timestamp,
            current_hover = ?self.current_hover,
            "started rendering timeline overlay",
        );

        ctx.clear_rect(0.0, 0.0, self.canvas_width, self.canvas_height);

        if let Some(&(ref hitbox, (x, y))) = self.current_hover.as_ref() {
            ctx.save();
            ctx.begin_path();
            ctx.move_to((x + 5) as f64, (y + 5) as f64);

            let text_info = ctx.measure_text(&hitbox.tooltip).unwrap();
            draw_rounded_rect(
                &ctx,
                x as f64,
                y as f64,
                text_info.width(),
                BOX_HEIGHT,
                RADIUS,
            )
            .unwrap();

            ctx.set_fill_style(&self.black);
            ctx.set_text_align("start");
            ctx.set_text_baseline("hanging");
            ctx.set_font("14px sans-serif");

            let label_x = ((x + 5) as f64).min(self.canvas_width - text_info.width() - X_LINE);

            ctx.fill_text(&hitbox.tooltip, label_x, y as f64 + BOX_HEIGHT / 2.0 - 5.0)
                .unwrap();

            ctx.restore();
        }

        tracing::trace!(
            timestamp = %timestamp,
            "finished rendering timeline overlay",
        );
    }

    fn draw_x_axis(&self, ctx: &CanvasRenderingContext2d, view: &ViewMap, quant: &ViewQuant) {
        ctx.set_fill_style(&self.background_color);
        ctx.fill_rect(0.0, 0.0, self.canvas_width, self.canvas_height);

        ctx.set_line_width(2.0);
        ctx.set_font(&*self.font);
        ctx.set_text_align(&*self.text_align);

        // Draw main axes
        ctx.begin_path();
        ctx.move_to(X_LINE, MARGIN);
        ctx.line_to(X_LINE, self.graph_height + MARGIN);
        ctx.line_to(X_LINE + self.graph_width + 20.0, self.graph_height + MARGIN);
        ctx.stroke();

        // Draw the x tick marks
        ctx.set_fill_style(&self.x_tick_color);

        ctx.begin_path();
        for time in self
            .view_range
            .clone()
            .step_by(quant.time_step as usize * 25)
            .skip(1)
        {
            let x = X_LINE + view.to_x(time);

            ctx.move_to(x, self.canvas_height - Y_LINE);
            ctx.line_to(x, self.canvas_height - Y_LINE + 7.0);

            ctx.save();
            ctx.translate(x, self.canvas_height - Y_LINE + 20.0)
                .unwrap();

            // Rotate the text so it doesn't overlap
            ctx.rotate(PI / 4.0).unwrap();

            // Remove trailing nanoseconds
            let time = Duration::from_nanos(time);
            // Totally not a hack, I swear
            let time = (time - Duration::from_nanos(time.subsec_nanos() as u64))
                + Duration::from_micros(time.subsec_micros() as u64);

            ctx.set_text_align("start");
            ctx.fill_text(
                // TODO: Use a format buffer
                &format!("{}", HumanDuration::from(time)),
                0.0,
                0.0,
            )
            .unwrap();

            ctx.restore();
        }
        ctx.stroke();

        // Draw vertical lines
        ctx.set_stroke_style(&self.vertical_stroke_style);
        ctx.set_line_dash(&self.vertical_line_dash).unwrap();

        ctx.begin_path();
        for time in self.view_range.clone().step_by(quant.time_step as usize) {
            let x = X_LINE + view.to_x((time + 1) * quant.time_step);
            ctx.move_to(x, MARGIN);
            ctx.line_to(x, MARGIN + self.graph_height);
        }
        ctx.stroke();

        ctx.set_stroke_style(&self.black);
        ctx.set_line_dash(&self.no_line_dash).unwrap();
    }

    pub(super) fn scale_timeline(&self) {
        // TODO: Use a format buffer
        let (canvas_width, canvas_height) = (
            self.canvas_width.to_string(),
            self.canvas_height.to_string(),
        );

        let container_style = self.graph_div.cast::<HtmlDivElement>().unwrap().style();

        container_style
            .set_property("width", &canvas_width)
            .unwrap();

        container_style
            .set_property("height", &canvas_height)
            .unwrap();
    }

    fn draw_y_line(
        &self,
        event: &TimelineEvent,
        idx: usize,
        max_idx: usize,
        ctx: &CanvasRenderingContext2d,
    ) {
        // Draw Y tick marks
        let y = self.graph_height - ((idx + 1) as f64 * Y_TICK_DIST);

        if idx != max_idx {
            ctx.begin_path();
            ctx.move_to(X_LINE, y);
            ctx.line_to(X_LINE - 5.0, y);
            ctx.stroke();
        }

        // Draw Y labels
        let y = MARGIN + (Y_TICK_DIST * (idx + 1) as f64) - 13.0;

        ctx.set_text_align("end");
        // TODO: Use a format buffer
        ctx.fill_text(&format!("{}", event), X_LINE - 4.0, y)
            .unwrap();
    }
}

#[derive(Debug)]
struct ViewMap {
    start: f64,
    scale: f64,
}

impl ViewMap {
    pub fn new(range: &Range<Nanos>, width: f64) -> Self {
        Self {
            start: range.start as f64,
            scale: width / ((range.end - range.start) as f64),
        }
    }

    pub fn to_x(&self, time: Nanos) -> f64 {
        ((time as f64) - self.start) * self.scale
    }

    // pub fn to_ns(&self, x: f64) -> f64 {
    //     self.start + (x / self.scale)
    // }
}

#[derive(Debug)]
struct ViewQuant {
    pub time_step: Nanos,
}

impl ViewQuant {
    pub fn new(range: &Range<Nanos>, width: f64) -> Self {
        let ns_per_px = (range.end - range.start) / (width as u64);
        let min_event_px = 2;
        let step = 1 << (64 - (ns_per_px * min_event_px).leading_zeros());

        Self {
            time_step: u64::max(1, step),
        }
    }

    pub fn round_down(&self, x: Nanos) -> Nanos {
        x - (x % self.time_step)
    }

    pub fn quantize(&self, range: &Range<Nanos>) -> Range<Nanos> {
        self.round_down(range.start)..(self.round_down(range.end) + self.time_step)
    }
}
