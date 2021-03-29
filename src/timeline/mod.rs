mod canvas;
mod constants;
mod properties;
mod render;
mod required_lines;
mod utils;

use crate::{
    data::WorkerTimelineEvent,
    timeline::{
        canvas::Canvas, properties::TimelineProps, required_lines::RequiredLines,
        utils::calculate_timeline_dimensions,
    },
};
use std::{borrow::Cow, rc::Rc};
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, MouseEvent};
use yew::{html, Component, ComponentLink, Html, NodeRef, ShouldRender};

#[derive(Debug)]
pub enum Message {
    RenderTimeline(CanvasRenderingContext2d, f64),
    RenderOverlay(CanvasRenderingContext2d, f64),
    MouseMove(MouseEvent),
    // CutoffPercent { percentage: usize },
}

#[derive(Debug, Clone)]
pub struct Hitbox {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    // TODO: Intern or Rc
    tooltip: String,
}

#[derive(Debug)]
pub struct Timeline {
    link: ComponentLink<Self>,
    events: Rc<[WorkerTimelineEvent]>,

    duration: f64,
    scale: f64,
    px_per_sec: f64,
    dpr: f64,
    required_lines: RequiredLines,

    graph_width: f64,
    graph_height: f64,
    graph_div: NodeRef,

    canvas_width: f64,
    canvas_height: f64,

    background_color: JsValue,
    font: Cow<'static, str>,
    text_align: Cow<'static, str>,
    x_tick_color: JsValue,
    vertical_stroke_style: JsValue,
    vertical_line_dash: JsValue,
    black: JsValue,
    no_line_dash: JsValue,
    block_rect_fill: JsValue,

    hitboxes: Vec<Hitbox>,
    // TODO: Make this a struct
    current_hover: Option<(Hitbox, (i32, i32))>,
}

impl Timeline {
    fn mouse_collision(&self, event: &MouseEvent) -> Option<(Hitbox, (i32, i32))> {
        let (x, y) = (event.offset_x(), event.offset_y());

        for hitbox in self.hitboxes.iter() {
            if x as f64 <= hitbox.x
                && x as f64 >= hitbox.x + hitbox.width
                && y as f64 <= hitbox.y
                && y as f64 >= hitbox.y + hitbox.height
            {
                return Some((hitbox.clone(), (x, y)));
            }
        }

        None
    }
}

impl Component for Timeline {
    type Message = Message;
    type Properties = TimelineProps;

    fn create(properties: Self::Properties, link: ComponentLink<Self>) -> Self {
        tracing::info!("creating timeline");

        let (
            scale,
            duration,
            graph_width,
            graph_height,
            px_per_sec,
            canvas_width,
            canvas_height,
            required_lines,
        ) = calculate_timeline_dimensions(&properties);

        let window = web_sys::window().unwrap();
        let dpr = window.device_pixel_ratio();

        Self {
            link,
            events: properties.events,

            scale,
            duration,
            px_per_sec,
            dpr,
            required_lines,

            graph_width,
            graph_height,
            graph_div: NodeRef::default(),

            canvas_width,
            canvas_height,

            background_color: JsValue::from_str("#F7F7F7"),
            font: "16px sans-serif".into(),
            text_align: "center".into(),
            x_tick_color: JsValue::from_str("#303030"),
            vertical_stroke_style: JsValue::from_str("#E6E6E6"),
            vertical_line_dash: JsValue::from_serde(&[2.0f64, 4.0]).unwrap(),
            black: JsValue::from_str("#000"),
            no_line_dash: JsValue::from_serde(&[] as &[f64]).unwrap(),
            block_rect_fill: JsValue::from_str("#95CCE8"),

            hitboxes: Vec::new(),
            current_hover: None,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        tracing::trace!("updating timeline");

        match message {
            Message::RenderTimeline(ref context, timestamp) => {
                //tracing::debug!("rendering timeline");
                self.render_timeline(context, timestamp);
            }

            Message::RenderOverlay(ref context, timestamp) => {
                //tracing::debug!("rendering overlay");
                self.render_overlay(context, timestamp);
            }

            Message::MouseMove(event) => {
                tracing::debug!("mouse move event");
                self.current_hover = self.mouse_collision(&event);
            }
        }

        false
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        tracing::info!(
            old = ?self.events,
            new = ?properties.events,
            "changing timeline",
        );

        let (
            scale,
            duration,
            graph_width,
            graph_height,
            px_per_sec,
            canvas_width,
            canvas_height,
            required_lines,
        ) = calculate_timeline_dimensions(&properties);

        self.scale = scale;
        self.duration = duration;
        self.graph_width = graph_width;
        self.graph_height = graph_height;
        self.px_per_sec = px_per_sec;
        self.canvas_width = canvas_width;
        self.canvas_height = canvas_height;
        self.required_lines = required_lines;
        self.events = properties.events;

        self.scale_timeline();

        true
    }

    fn view(&self) -> Html {
        tracing::info!("viewing timeline");

        let render_timeline: Rc<dyn Fn(&CanvasRenderingContext2d, f64)> = {
            let link = self.link.clone();

            Rc::new(move |ctx, timestamp| {
                link.send_message(Message::RenderTimeline(ctx.clone(), timestamp));
            })
        };

        let render_overlay: Rc<dyn Fn(&CanvasRenderingContext2d, f64)> = {
            let link = self.link.clone();

            Rc::new(move |ctx, timestamp| {
                link.send_message(Message::RenderOverlay(ctx.clone(), timestamp));
            })
        };

        html! {
            <div id="timeline" ref=self.graph_div.clone() onmousemove=self.link.callback(Message::MouseMove)>
                // TODO: Allow configuring the cutoff percent of events
                // <input
                //     type="range"
                //     min="0"
                //     max="100"
                //     value="10"
                //     id="timeline-cutoff-percentage"
                //     onchange=self.link.callback(|value| Message::CutoffPercent)
                // />

                <Canvas
                    id={ Rc::from("timeline-canvas") }
                    class={ Rc::from("canvas-layer") }
                    width={ self.canvas_width.ceil() as u32 }
                    height={ self.canvas_height.ceil() as u32 }
                    render={ render_timeline }
                />

                <Canvas
                    id={ Rc::from("timeline-overlay-canvas") }
                    class={ Rc::from("canvas-layer") }
                    width={ self.canvas_width.ceil() as u32 }
                    height={ self.canvas_height.ceil() as u32 }
                    render={ render_overlay }
                />
            </div>
        }
    }

    fn rendered(&mut self, is_first_render: bool) {
        tracing::info!(is_first_render = is_first_render, "rendering timeline");
        self.scale_timeline();
    }
}