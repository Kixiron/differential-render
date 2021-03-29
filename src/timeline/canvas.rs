use std::{
    f64,
    fmt::{self, Debug, Write},
    rc::Rc,
};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use yew::{
    html,
    services::{render::RenderTask, RenderService},
    Component, ComponentLink, Html, NodeRef, Properties, ShouldRender,
};

#[derive(Debug)]
pub struct Canvas {
    link: ComponentLink<Self>,
    context: Option<CanvasRenderingContext2d>,
    canvas: NodeRef,
    properties: CanvasProperties,
    render_handle: Option<RenderTask>,
    dpr: f64,
    buffer: String,
}

impl Canvas {
    fn register_render(&mut self) {
        tracing::trace!(
            "registering a render callback for canvas #{}",
            self.properties.id,
        );

        let render_frame = self.link.callback(CanvasMessage::Render);
        let handle = RenderService::request_animation_frame(render_frame);

        self.render_handle = Some(handle);
    }

    fn scale_canvas(&mut self, canvas: Option<&HtmlCanvasElement>) {
        if let Some(context) = self.context.as_ref() {
            tracing::trace!("scaled canvas #{}", self.properties.id);

            let canvas = canvas
                .cloned()
                .unwrap_or_else(|| self.canvas.cast::<HtmlCanvasElement>().unwrap());

            // Scale the rendering context to the device's pixel radio
            context.scale(self.dpr, self.dpr).unwrap();
            canvas.set_width((self.properties.width as f64 * self.dpr).ceil() as u32);
            canvas.set_height((self.properties.height as f64 * self.dpr).ceil() as u32);

            let canvas_style = canvas.style();

            self.buffer.clear();
            write!(&mut self.buffer, "{}", self.properties.width).unwrap();
            canvas_style.set_property("width", &self.buffer).unwrap();

            self.buffer.clear();
            write!(&mut self.buffer, "{}", self.properties.height).unwrap();
            canvas_style.set_property("height", &self.buffer).unwrap();
        } else {
            tracing::error!(
                "attempted to scale canvas #{} without a rendering context",
                self.properties.id,
            );
        }
    }

    fn create_canvas(&mut self) {
        let canvas = self.canvas.cast::<HtmlCanvasElement>().unwrap();
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        self.context = Some(context);

        self.scale_canvas(Some(&canvas));
    }
}

impl Component for Canvas {
    type Message = CanvasMessage;
    type Properties = CanvasProperties;

    fn create(properties: Self::Properties, link: ComponentLink<Self>) -> Self {
        tracing::debug!(properties = ?properties, "creating canvas #{}", properties.id);

        let window = web_sys::window().unwrap();
        let dpr = window.device_pixel_ratio();

        Self {
            link,
            context: None,
            canvas: NodeRef::default(),
            properties,
            render_handle: None,
            dpr,
            buffer: String::new(),
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        tracing::trace!(message = ?message, "updating canvas #{}", self.properties.id);

        match message {
            CanvasMessage::Render(timestamp) => {
                if let Some(callback) = self.properties.render.as_deref() {
                    tracing::trace!("rendering canvas #{}", self.properties.id);

                    let context = self.context.as_ref().unwrap();

                    let span = tracing::debug_span!("render callback for canvas", canvas_id = %self.properties.id, );
                    span.in_scope(|| (callback)(context, timestamp));
                }

                self.register_render();
            }
        }

        false
    }

    fn change(&mut self, properties: Self::Properties) -> ShouldRender {
        tracing::debug!(
            old_properties = ?self.properties,
            new_properties = ?properties,
            "changing canvas #{}",
            self.properties.id,
        );

        self.properties = properties;
        self.scale_canvas(None);

        false
    }

    fn view(&self) -> Html {
        tracing::trace!("building view for canvas #{}", self.properties.id);

        html! {
            <canvas
                id={ self.properties.id.clone() }
                class={ &*self.properties.class }
                ref=self.canvas.clone()
            >
            </canvas>
        }
    }

    fn rendered(&mut self, is_first_render: bool) {
        tracing::trace!(
            is_first_render = is_first_render,
            "rendering canvas #{}",
            self.properties.id,
        );

        self.create_canvas();

        if is_first_render {
            self.register_render();
        }
    }
}

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    Render(f64),
}

fn empty_str() -> Rc<str> {
    Rc::from("")
}

#[derive(Clone, Properties)]
pub struct CanvasProperties {
    #[prop_or_else(empty_str)]
    pub id: Rc<str>,

    #[prop_or_else(empty_str)]
    pub class: Rc<str>,

    #[prop_or(500)]
    pub width: u32,

    #[prop_or(500)]
    pub height: u32,

    #[prop_or_default]
    pub render: Option<Rc<dyn Fn(&CanvasRenderingContext2d, f64)>>,
}

impl Debug for CanvasProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CanvasProperties")
            .field("id", &self.id)
            .field("class", &self.class)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("render", &self.render.as_ref().map(Rc::as_ptr))
            .finish()
    }
}
