use crate::{
    data::TimelineEvent,
    timeline::{
        constants::{BOX_HEIGHT, MARGIN, RADIUS, X_LINE, Y_LINE, Y_TICK_DIST},
        utils::{draw_rounded_rect, split_ticks},
        Hitbox, Timeline,
    },
};
use humantime::Duration as HumanDuration;
use std::{
    collections::{hash_map::Entry, HashMap},
    f64::consts::PI,
    time::Duration,
};
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

        // Draw the graph's axis
        self.draw_x_axis(ctx);

        // TODO: Persistent buffer
        let mut line_coords = HashMap::with_capacity(self.required_lines.required_lines());
        let mut line_indices = (0..self.required_lines.required_lines()).rev();

        // Make sure that important events occur at the beginning relative to operator invocations
        if self.required_lines.park_occurs() {
            let idx = line_indices.next().unwrap();

            line_coords.insert(&TimelineEvent::Parked, idx);
            self.draw_y_line(&TimelineEvent::Parked, idx, ctx);
        }

        if self.required_lines.message_occurs() {
            let idx = line_indices.next().unwrap();

            line_coords.insert(&TimelineEvent::Message, idx);
            self.draw_y_line(&TimelineEvent::Parked, idx, ctx);
        }

        if self.required_lines.progress_occurs() {
            let idx = line_indices.next().unwrap();

            line_coords.insert(&TimelineEvent::Progress, idx);
            self.draw_y_line(&TimelineEvent::Parked, idx, ctx);
        }

        if self.required_lines.application_occurs() {
            let idx = line_indices.next().unwrap();

            line_coords.insert(&TimelineEvent::Application, idx);
            self.draw_y_line(&TimelineEvent::Parked, idx, ctx);
        }

        self.hitboxes.clear();
        self.hitboxes.reserve(self.events.len());

        for event in self.events.iter() {
            let line_idx = match line_coords.entry(&event.event) {
                Entry::Occupied(occupied) => *occupied.get(),

                Entry::Vacant(vacant) => {
                    let idx = *vacant.insert(line_indices.next().unwrap());
                    self.draw_y_line(&event.event, idx, ctx);

                    idx
                }
            };

            ctx.save();
            ctx.translate(X_LINE, MARGIN).unwrap();

            let y = line_idx as f64 * Y_TICK_DIST + 1.0;
            let x = self.px_per_sec * event.start_time();
            let width = (self.px_per_sec * event.duration()).max(1.0);
            let height = BOX_HEIGHT;

            ctx.begin_path();
            ctx.set_fill_style(&self.block_rect_fill);
            draw_rounded_rect(ctx, x, y, width, height, RADIUS).unwrap();
            ctx.fill();

            ctx.set_fill_style(&self.black);
            ctx.set_text_align("start");
            ctx.set_text_baseline("hanging");
            ctx.set_font("14px sans-serif");

            // TODO: Format buffer
            let label = format!(
                "{}",
                HumanDuration::from(Duration::from_millis(event.duration().floor() as u64)),
            );
            let text_info = ctx.measure_text(&label).unwrap();
            let label_x = (x + 5.0).min(self.canvas_width - text_info.width() - X_LINE);

            ctx.fill_text(&label, label_x, y + height / 2.0 - 5.0)
                .unwrap();

            ctx.restore();

            self.hitboxes.push(Hitbox {
                x: x + X_LINE,
                y: y + MARGIN,
                width,
                height,
                tooltip: format!("{} ran for {}", &event.event, &label),
            });
        }

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

    fn draw_x_axis(&self, ctx: &CanvasRenderingContext2d) {
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
        let (step, tick_dist, num_ticks) =
            split_ticks(self.duration, self.px_per_sec, self.graph_width);

        ctx.set_fill_style(&self.x_tick_color);

        ctx.begin_path();
        for n in 0..num_ticks {
            let x = X_LINE + ((n + 1) as f64 * tick_dist);

            ctx.move_to(x, self.canvas_height - Y_LINE);
            ctx.line_to(x, self.canvas_height - Y_LINE + 7.0);

            ctx.save();
            ctx.translate(x, self.canvas_height - Y_LINE + 20.0)
                .unwrap();

            // Rotate the text so it doesn't overlap
            ctx.rotate(PI / 4.0).unwrap();

            // TODO: Use a format buffer
            ctx.set_text_align("start");
            ctx.fill_text(
                &format!(
                    "{}",
                    HumanDuration::from(Duration::from_millis(
                        ((n + 1) as f64 * step).floor() as u64
                    )),
                ),
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
        for n in 0..num_ticks {
            let x = X_LINE + ((n + 1) as f64 * tick_dist);
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

    fn draw_y_line(&self, event: &TimelineEvent, idx: usize, ctx: &CanvasRenderingContext2d) {
        // Draw Y tick marks
        let y = self.graph_height - ((idx + 1) as f64 * Y_TICK_DIST);

        ctx.begin_path();
        ctx.move_to(X_LINE, y);
        ctx.line_to(X_LINE - 5.0, y);
        ctx.stroke();

        // Draw Y labels
        let y = MARGIN + (Y_TICK_DIST * (idx + 1) as f64) - 13.0;

        ctx.set_text_align("end");
        // TODO: Use a format buffer
        ctx.fill_text(&format!("{}", event), X_LINE - 4.0, y)
            .unwrap();
    }
}
