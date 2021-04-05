use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    convert::TryInto,
    fmt::{self, Display},
};

pub const NS_MARGIN: u64 = 500_000;

pub const NS_TO_MS: f64 = 1_000_000.0; // ns->sec = 1_000_000_000.0;

pub type EventId = u64;

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct ProfilingData {
    pub nodes: Vec<Node>,
    pub subgraphs: Vec<Subgraph>,
    pub edges: Vec<Edge>,
    pub palette_colors: Vec<String>,
    pub timeline_events: Vec<WorkerTimelineEvent>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Node {
    pub id: usize,
    pub addr: Vec<usize>,
    pub name: String,
    pub max_activation_time: String,
    pub min_activation_time: String,
    pub average_activation_time: String,
    pub total_activation_time: String,
    pub invocations: usize,
    pub fill_color: String,
    pub text_color: String,
    pub activation_durations: Vec<ActivationDuration>,
    pub max_arrangement_size: Option<usize>,
    pub min_arrangement_size: Option<usize>,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct ActivationDuration {
    pub activation_time: u64,
    pub activated_at: u64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Subgraph {
    pub id: usize,
    pub addr: Vec<usize>,
    pub name: String,
    pub max_activation_time: String,
    pub mix_activation_time: String,
    pub average_activation_time: String,
    pub total_activation_time: String,
    pub invocations: usize,
    pub fill_color: String,
    pub text_color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Edge {
    pub src: Vec<usize>,
    pub dest: Vec<usize>,
    pub channel_id: usize,
    pub edge_kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum EdgeKind {
    Normal,
    Crossing,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct WorkerTimelineEvent {
    pub event_id: EventId,
    pub worker: usize,
    pub event: TimelineEvent,
    pub start_time: u64,
    pub duration: u64,
    /// The number of events that have been collapsed within the current timeline event
    pub collapsed_events: usize,
}

impl WorkerTimelineEvent {
    pub const fn end_time(&self) -> u64 {
        self.start_time + self.duration
    }

    pub fn start_time(&self) -> f64 {
        self.start_time as f64 / NS_TO_MS
    }

    pub fn duration(&self) -> f64 {
        self.duration as f64 / NS_TO_MS
    }

    pub fn overlap(&self, other: &Self) -> Ordering {
        // If the ranges overlap they're considered equal
        if self.start_time + NS_MARGIN <= other.end_time()
            && self.end_time() + NS_MARGIN >= other.start_time
        {
            Ordering::Equal
        } else if self.start_time + NS_MARGIN < other.start_time
            && self.end_time() + NS_MARGIN < other.end_time()
        {
            Ordering::Greater
        } else if self.start_time + NS_MARGIN > other.start_time
            && self.end_time() + NS_MARGIN > other.end_time()
        {
            Ordering::Less
        } else {
            unreachable!()
        }
    }

    /*
    pub fn text_overlap(&self, other: &Self, ctx: &CanvasRenderingContext2d) -> Ordering {
        let label = self.event.to_string();
        let other_label = other.event.to_string();

        let self_end = ((self.start_time + NS_MARGIN) as f64)
            .max(ctx.measure_text(&label).unwrap().width() + NS_MARGIN as f64);
        let other_end = ((other.start_time + NS_MARGIN) as f64)
            .max(ctx.measure_text(&other_label).unwrap().width() + NS_MARGIN as f64);

        if (self.start_time + NS_MARGIN) as f64 <= other_end && self_end >= other.start_time as f64
        {
            Ordering::Equal
        } else if self.start_time + NS_MARGIN < other.start_time
            && self.end_time() + NS_MARGIN < other.end_time()
        {
            Ordering::Greater
        } else if self.start_time + NS_MARGIN > other.start_time
            && self.end_time() + NS_MARGIN > other.end_time()
        {
            Ordering::Less
        } else {
            unreachable!()
        }
    }*/
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimelineEvent {
    OperatorActivation {
        operator_id: usize,
        operator_name: String,
    },
    Merge {
        operator_id: usize,
        operator_name: String,
    },
    Progress,
    Message,
    Application,
    Input,
    Parked,
}

impl TimelineEvent {
    fn rank(&self) -> u128 {
        let mut bytes = 0u128.to_ne_bytes();

        let [first, second] = match self {
            &Self::OperatorActivation { operator_id, .. } => [operator_id as u64, 6],
            &Self::Merge { operator_id, .. } => [operator_id as u64, 5],
            Self::Progress => [0, 4],
            Self::Message => [0, 3],
            Self::Application => [0, 2],
            Self::Input => [0, 1],
            Self::Parked => [0, 0],
        };

        bytes[..8].copy_from_slice(&first.to_le_bytes());
        bytes[8..].copy_from_slice(&second.to_le_bytes());
        u128::from_le_bytes(bytes)
    }
}

impl PartialOrd for TimelineEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.rank().partial_cmp(&other.rank())
    }
}

impl Ord for TimelineEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl Display for TimelineEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OperatorActivation {
                operator_id,
                operator_name,
            } => {
                write!(f, "Operator {}: {}", operator_id, operator_name)
            }
            Self::Application => f.write_str("Application"),
            Self::Parked => f.write_str("Parked"),
            Self::Input => f.write_str("Input"),
            Self::Message => f.write_str("Message"),
            Self::Progress => f.write_str("Progress"),
            Self::Merge {
                operator_id,
                operator_name,
            } => write!(f, "Merge {}: {}", operator_id, operator_name),
        }
    }
}
