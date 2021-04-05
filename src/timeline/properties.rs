use crate::data::WorkerTimelineEvent;
use std::rc::Rc;
use yew::Properties;

#[derive(Debug, Clone, Properties)]
pub struct TimelineProps {
    pub events: Rc<[WorkerTimelineEvent]>,
    pub duration: f64,
    pub scale: f64,
    pub event_cutoff: u64,
}
