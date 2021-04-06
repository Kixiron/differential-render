use crate::trace::{BlockPool, Nanos, TraceBlock, TraceEvent, Track};
use core::fmt::Debug;

pub trait Aggregate<K> {
    fn empty() -> Self;

    fn is_empty(&self) -> bool
    where
        Self: Sized + PartialEq,
    {
        self == &Self::empty()
    }

    fn from_event(event: &TraceEvent<K>) -> Self;

    fn join(&self, other: &Self) -> Self;

    fn from_block(block: &TraceBlock<K>) -> Self
    where
        Self: Sized,
        K: Default,
    {
        let mut aggregate = Self::empty();
        for event in block.events() {
            aggregate = Self::join(&aggregate, &Self::from_event(event));
        }

        aggregate
    }
}

pub trait TrackIndex<K: Default, A: Aggregate<K>> {
    fn build(track: &Track<K>, pool: &BlockPool<K>) -> Self;
}

// === Concrete aggregations

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LongestEvent<K>(pub Option<TraceEvent<K>>);

impl<K: Default + Clone + Debug> Aggregate<K> for LongestEvent<K> {
    fn empty() -> Self {
        LongestEvent(None)
    }

    fn from_event(event: &TraceEvent<K>) -> Self {
        LongestEvent(Some(event.clone()))
    }

    fn join(&self, other: &Self) -> Self {
        LongestEvent(
            [self.0.clone(), other.0.clone()]
                .iter()
                .filter_map(Option::as_ref)
                .max_by_key(|event| event.duration.unpack())
                .cloned(),
        )
    }

    fn from_block(block: &TraceBlock<K>) -> Self {
        LongestEvent(
            block
                .events()
                .iter()
                .max_by_key(|event| event.duration.unpack())
                .cloned(),
        )
    }
}

// #[derive(Clone)]
// pub struct LongestEventLoc {
//     dur: Ns,
//     index: usize,
// }
//
// impl Aggregate for LongestEventLoc {
//     fn empty() -> Self {
//         LongestEventLoc { dur: 0, index: usize::MAX }
//     }
//
//     fn from_event(ev: &TraceEvent) -> Self {
//         LongestEventLoc(Some(ev.clone()))
//     }
//
//     fn combine(&self, other: &Self) -> Self {
//         LongestEventLoc([self.0, other.0].iter()
//             .filter_map(|x| x.as_ref())
//             .max_by_key(|ev| ev.dur.unpack())
//             .map(|x| x.clone()))
//     }
//
//     fn from_block(block: &TraceBlock) -> Self {
//         LongestEventLoc(block.events().iter().max_by_key(|ev| ev.dur.unpack()).map(|x| x.clone()))
//     }
// }

/// For debugging
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct EventCount(pub Nanos);

impl<K: Default> Aggregate<K> for EventCount {
    fn empty() -> Self {
        EventCount(0)
    }

    fn from_event(_event: &TraceEvent<K>) -> Self {
        EventCount(1)
    }

    fn join(&self, other: &Self) -> Self {
        EventCount(self.0 + other.0)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct EventSum(pub Nanos);

impl<K: Default> Aggregate<K> for EventSum {
    fn empty() -> Self {
        Self(0)
    }

    fn from_event(event: &TraceEvent<K>) -> Self {
        Self(event.timestamp.unpack())
    }

    fn join(&self, other: &Self) -> Self {
        Self(self.0 + other.0)
    }
}
