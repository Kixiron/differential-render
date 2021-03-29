use crate::data::{TimelineEvent, WorkerTimelineEvent};
use std::collections::HashMap;

type MultiSet<K, V> = HashMap<K, V>;

#[derive(Debug, Clone)]
pub(super) struct RequiredLines {
    unique_events: MultiSet<EventKind, u32>,
}

impl RequiredLines {
    pub fn new(events: &[WorkerTimelineEvent]) -> Self {
        let mut this = Self::with_capacity(events.len() / 2);

        for event in events {
            this.add(event);
        }

        this
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            unique_events: MultiSet::with_capacity(capacity),
        }
    }

    pub fn required_lines(&self) -> usize {
        self.unique_events.len()
    }

    pub fn add(&mut self, event: &WorkerTimelineEvent) {
        self.unique_events
            .entry(EventKind::from(&event.event))
            .and_modify(|diff| *diff += 1)
            .or_insert(1);
    }

    pub fn remove(&mut self, event: &WorkerTimelineEvent) {
        self.unique_events
            .entry(EventKind::from(&event.event))
            .and_modify(|diff| *diff -= 1);
    }

    pub fn park_occurs(&self) -> bool {
        self.unique_events.contains_key(&EventKind::Parked)
    }

    pub fn message_occurs(&self) -> bool {
        self.unique_events.contains_key(&EventKind::Message)
    }

    pub fn progress_occurs(&self) -> bool {
        self.unique_events.contains_key(&EventKind::Progress)
    }

    pub fn application_occurs(&self) -> bool {
        self.unique_events.contains_key(&EventKind::Application)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum EventKind {
    OperatorActivation { operator_id: usize },
    Application,
    Parked,
    Input,
    Message,
    Progress,
    Merge { operator_id: usize },
}

impl From<&TimelineEvent> for EventKind {
    fn from(event: &TimelineEvent) -> Self {
        match *event {
            TimelineEvent::OperatorActivation { operator_id, .. } => {
                Self::OperatorActivation { operator_id }
            }
            TimelineEvent::Merge { operator_id, .. } => Self::Merge { operator_id },
            TimelineEvent::Application => Self::Application,
            TimelineEvent::Parked => Self::Parked,
            TimelineEvent::Input => Self::Input,
            TimelineEvent::Message => Self::Message,
            TimelineEvent::Progress => Self::Progress,
        }
    }
}
