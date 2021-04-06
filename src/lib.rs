#![recursion_limit = "256"]

// TODO: God this needs a refactor so bad, just every bit of it
// TODO: We need to take advantage of incrementality here so that
//       we can handle realtime streaming data at 60fps
// TODO: Nothing here is really tied to WASM at all, maybe look
//       into a UI lib that allows being abstracted from the renderer
//       and takes care of platform woes so that we can have a web &
//       native client, druid looks promising and pretty similar to
//       the yew setup, so that could be reasonably easy to port to

mod data;
mod timeline;

use crate::{
    data::{ProfilingData, WorkerTimelineEvent},
    timeline::{constants::NS_TO_MS, Timeline},
};
use anyhow::{Context, Error, Result};
use std::{cmp::Ordering, rc::Rc};
use tracing::Level;
use tracing_wasm::WASMLayerConfigBuilder;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use web_sys::{File, HtmlParagraphElement};
use yew::{
    events::{ChangeData, InputData},
    html,
    services::{
        reader::{FileData, ReaderTask},
        storage::Area,
        ReaderService, StorageService,
    },
    Component, ComponentLink, Html, NodeRef, ShouldRender,
};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const LAST_FILE_KEY: &str = "differential-dashboard.last-opened-file";

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
    tracing_wasm::set_as_global_default_with_config(
        WASMLayerConfigBuilder::new()
            .set_max_level(Level::DEBUG)
            .build(),
    );

    yew::start_app::<Dashboard>();

    Ok(())
}

#[derive(Debug)]
struct Dashboard {
    link: ComponentLink<Self>,
    file_task: Option<(ReaderTask, File)>,
    events: Rc<[WorkerTimelineEvent]>,
    alerts: Vec<Alert>,
    storage: Option<StorageService>,
    cutoff_display: NodeRef,
    cutoff_value: u64,
}

impl Dashboard {
    fn dispatch(&mut self, message: Message) -> ShouldRender {
        match message {
            Message::LoadFile { file } => {
                if let Some((_, file)) = self.file_task.as_ref() {
                    self.alerts
                        .push(Alert::LoadFileCanceled { file: file.clone() })
                }

                match self.load_file(file.clone()) {
                    Ok(task) => {
                        self.file_task = Some((task, file));
                        false
                    }

                    Err(error) => {
                        self.alerts.push(Alert::FailedFetch { file, error });
                        true
                    }
                }
            }

            Message::FileReady { data } => {
                // Discard the task once it's completed
                let _ = self.file_task.take();

                match data {
                    Ok(mut data) => {
                        tracing::info!("loaded file data");
                        data.timeline_events
                            .sort_unstable_by_key(|event| event.event.clone());

                        self.events = Rc::from(data.timeline_events);
                        true
                    }

                    Err(error) => {
                        self.alerts.push(Alert::generic(error));
                        true
                    }
                }
            }

            Message::ChangeCutoff(cutoff) => {
                self.cutoff_value = cutoff;

                if let Some(cutoff_display) = self.cutoff_display.cast::<HtmlParagraphElement>() {
                    // TODO: Buffer this?
                    cutoff_display.set_inner_text(&format!("Event time cutoff: {}ms", cutoff));
                }

                true
            }
        }
    }

    fn load_file(&mut self, file: File) -> Result<ReaderTask> {
        let callback = self.link.callback(move |file_data: FileData| {
            let data = serde_json::from_slice(&file_data.content).map_err(Into::into);
            tracing::debug!("Loaded file {:?}\nContent: {:?}", file_data.name, data);

            Message::FileReady { data }
        });

        ReaderService::new()
            .read_file(file.clone(), callback)
            .with_context(|| format!("failed fetching file {}", file.name()))
    }
}

impl Component for Dashboard {
    type Message = Message;
    type Properties = ();

    fn create(_properties: Self::Properties, link: ComponentLink<Self>) -> Self {
        let storage = StorageService::new(Area::Local).ok();

        if let Some(_file) = storage
            .as_ref()
            .and_then(|storage| storage.restore::<Result<String>>(LAST_FILE_KEY).ok())
        {
            // TODO: Load the most recently loaded file
            // FetchService::fetch(Request::get(file), |response| {})
        }

        Self {
            link,
            file_task: None,
            alerts: Vec::new(),
            events: Rc::from(Vec::new()),
            storage,
            cutoff_display: NodeRef::default(),
            cutoff_value: 0,
        }
    }

    fn update(&mut self, message: Self::Message) -> ShouldRender {
        self.dispatch(message)
    }

    fn change(&mut self, _properties: Self::Properties) -> ShouldRender {
        todo!()
    }

    fn view(&self) -> Html {
        let duration = self
            .events
            .iter()
            .map(|event| event.end_time() as f64 / NS_TO_MS)
            .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Less))
            .unwrap_or(0.0);

        html! {
            <>
                <div id="menu">
                    <p>{ "Choose a file to visualize profile events for" }</p>
                    <input type="file" multiple=false accept=".json,.json5" onchange=self.link.callback(move |change_data| {
                            if let ChangeData::Files(files) = change_data {
                                // TODO: Can the user give no files?
                                debug_assert_eq!(files.length(), 1);

                                Message::LoadFile { file: files.get(0).unwrap() }
                            } else {
                                unreachable!()
                            }
                        })
                    />

                    <p ref=self.cutoff_display.clone()>{ format!("Event time cutoff: {}ms", self.cutoff_value) }</p>
                    <input type="range" min="0" max="10000" step="1" value={ self.cutoff_value } oninput=self.link.callback(move |input_data: InputData| {
                            Message::ChangeCutoff(input_data.value.parse().unwrap())
                        })
                    />
                </div>

                { self.alerts.iter().map(Alert::render).collect::<Html>() }

                <Timeline
                    events=self.events.clone()
                    duration=duration
                    scale=50.0
                    event_cutoff=self.cutoff_value
                />
            </>
        }
    }

    fn rendered(&mut self, _is_first_render: bool) {
        self.alerts.clear();
    }
}

#[derive(Debug)]
enum Alert {
    LoadFileCanceled { file: File },
    FailedFetch { file: File, error: Error },
    Generic(String),
}

impl Alert {
    fn generic<T>(error: T) -> Self
    where
        T: ToString,
    {
        Self::Generic(error.to_string())
    }

    // TODO: Make this a full-blown component with modal popups
    fn render(&self) -> Html {
        let content = match self {
            Self::LoadFileCanceled { file } => format!("Canceled loading {}", file.name()),
            Self::FailedFetch { file, error } => {
                format!("Failed to load {}: {}", file.name(), error)
            }
            Self::Generic(message) => message.clone(),
        };

        html! {
            <div class="alert-popup">
                { content }
            </div>
        }
    }
}

#[derive(Debug)]
pub enum Message {
    LoadFile { file: File },
    FileReady { data: Result<ProfilingData> },
    ChangeCutoff(u64),
}
