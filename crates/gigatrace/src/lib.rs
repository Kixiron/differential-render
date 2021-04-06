pub mod iforest;
pub mod index;
pub mod trace;

use crate::{
    iforest::IForestIndex,
    index::{Aggregate, LongestEvent},
    trace::{BlockIndex, BlockPool, Nanos, Track},
};
use core::{convert::identity, mem, ops::Range};

// TODO: Preallocate
pub fn aggregate_by_steps<K, A, F>(
    pool: &BlockPool<K>,
    block_locations: &[BlockIndex],
    index: &IForestIndex<K, A>,
    time_span: Range<Nanos>,
    time_step: u64,
    event_filter: F,
) -> Vec<A>
where
    K: Default,
    A: Aggregate<K> + PartialEq + Clone,
    F: Fn(&A) -> bool,
{
    // TODO: Preallocate
    let mut out = vec![];

    let mut block_idx = 0;
    let mut target_time = time_span.start;
    let mut combined = A::empty();

    'outer: loop {
        if block_idx >= block_locations.len() {
            break;
        }

        // == Skip to last block with a start_time before target_time
        let search_result = block_locations[block_idx..]
            .binary_search_by_key(&target_time, |&idx| pool.blocks[idx as usize].start_time())
            .unwrap_or_else(identity);

        if search_result > 1 {
            let skip = search_result - 1;

            // == aggregate range using the index
            combined = A::join(&combined, &index.range_query(block_idx..(block_idx + skip)));

            block_idx += skip;
        }

        let block = &pool.blocks[block_locations[block_idx] as usize];
        for event in block.events() {
            let event_time = event.timestamp.unpack();

            while event_time >= target_time {
                // Only produce the aggregate if it's actual data
                let produced = mem::replace(&mut combined, A::empty());
                if !produced.is_empty() && event_filter(&produced) {
                    out.push(produced);
                }

                if target_time >= time_span.end {
                    break 'outer;
                }

                target_time += time_step;
            }

            combined = A::join(&combined, &A::from_event(event));
        }

        block_idx += 1;
    }

    // Only produce the aggregate if it's actual data
    if !combined.is_empty() && event_filter(&combined) {
        out.push(combined);
    }

    out
}

// TODO: Preallocate
pub fn aggregate_by_steps_unindexed<K, A, F>(
    pool: &BlockPool<K>,
    block_locs: &[BlockIndex],
    time_span: Range<Nanos>,
    time_step: u64,
    event_filter: F,
) -> Vec<A>
where
    K: Default,
    A: Aggregate<K> + PartialEq,
    F: Fn(&A) -> bool,
{
    // TODO: Preallocate
    let mut out = vec![];

    let mut target_time = time_span.start;
    let mut combined = A::empty();

    'outer: for &block_idx in block_locs {
        let block = &pool.blocks[block_idx as usize];

        for event in block.events() {
            let event_time = event.timestamp.unpack();

            while event_time >= target_time {
                // Only produce the aggregate if it's actual data
                let produced = mem::replace(&mut combined, A::empty());
                if !produced.is_empty() && event_filter(&produced) {
                    out.push(produced);
                }

                if target_time >= time_span.end {
                    break 'outer;
                }

                target_time += time_step;
            }

            combined = A::join(&combined, &A::from_event(event));
        }
    }

    // Only produce the aggregate if it's actual data
    if !combined.is_empty() && event_filter(&combined) {
        out.push(combined);
    }

    out
}

#[derive(Debug)]
pub struct TrackInfo<K> {
    pub track: Track<K>,
    pub zoom_index: IForestIndex<K, LongestEvent<K>>,
}

impl<K> TrackInfo<K> {
    pub const fn new(track: Track<K>, zoom_index: IForestIndex<K, LongestEvent<K>>) -> Self {
        Self { track, zoom_index }
    }
}

#[derive(Debug)]
pub struct Trace<K>
where
    K: Default,
{
    pub pool: BlockPool<K>,
    pub tracks: Vec<TrackInfo<K>>,
}

impl<K> Trace<K>
where
    K: Default,
{
    pub fn new() -> Self {
        Trace {
            pool: BlockPool::new(),
            tracks: Vec::new(),
        }
    }

    pub fn time_bounds(&self) -> Option<Range<Nanos>> {
        let start = self
            .tracks
            .iter()
            .filter_map(|track| track.track.start_time(&self.pool))
            .min();

        let end = self
            .tracks
            .iter()
            .filter_map(|track| track.track.after_last_time(&self.pool))
            .max();

        match (start, end) {
            (Some(start), Some(end)) => Some(start..end),
            (_, _) => None,
        }
    }
}

impl<K> Default for Trace<K>
where
    K: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        iforest::IForestIndex,
        index::{Aggregate, EventCount, EventSum, LongestEvent, TrackIndex},
        trace::{BlockIndex, BlockPool, Nanos, PackedNanos, TraceEvent, Track},
    };

    #[test]
    fn dummy_trace() {
        let mut pool = BlockPool::new();
        let mut track = Track::new();
        let rng = Rng::new();
        track.add_dummy_events(&mut pool, &rng, 300);

        let mut maxes = vec![];
        for i in &track.block_locations {
            maxes.push(
                LongestEvent::from_block(&pool.blocks[*i as usize])
                    .0
                    .unwrap()
                    .duration
                    .unpack(),
            );
        }
        println!("maxes: {:?}", maxes);

        let index = IForestIndex::<LongestEvent>::build(&track, &pool);
        let index_vals = index
            .values
            .iter()
            .map(|x| x.0.unwrap().duration.unpack())
            .collect::<Vec<_>>();
        println!("index: {:?}", index_vals);

        // assert!(false);
        // TODO some test
    }

    #[test]
    fn block_count() {
        let mut pool = BlockPool::new();
        let mut track = Track::new();
        let rng = Rng::new();
        track.add_dummy_events(&mut pool, &rng, 325);

        let index = IForestIndex::<EventCount>::build(&track, &pool);
        let index_vals = index.values.iter().map(|x| x.0).collect::<Vec<_>>();
        println!("index: {:?}", index_vals);

        // assert!(false);
        // TODO some test
    }

    #[test]
    fn aggregate_by_steps_unindexed() {
        let mut pool = BlockPool::new();
        let mut track = Track::new();
        let ev_ts = &[10, 15, 20, 100, 101, 150, 170];
        for t in ev_ts {
            track.push(
                &mut pool,
                TraceEvent {
                    kind: 0,
                    timestamp: PackedNanos::new(*t),
                    duration: PackedNanos::new(0),
                },
            );
        }

        let span = 13..150;
        let res = crate::aggregate_by_steps_unindexed::<EventSum>(
            &pool,
            &track.block_locations,
            span,
            10,
        );
        let res_ts = res.iter().map(|x| x.0).collect::<Vec<_>>();
        assert_eq!(
            &res_ts[..],
            &[10, 35, 0, 0, 0, 0, 0, 0, 0, 201, 0, 0, 0, 0, 150]
        );
    }

    #[test]
    fn prop_test_range_query() {
        let mut pool = BlockPool::new();
        let mut track = Track::new();
        let rng = Rng::new();
        track.add_dummy_events(&mut pool, &rng, 325);

        let index = IForestIndex::<EventCount>::build(&track, &pool);
        // let index_vals = index.vals.iter().map(|x| x.0).collect::<Vec<_>>();
        // println!("index: {:?}", index_vals);

        for _ in 0..100_000 {
            let start = rng.usize(..=track.block_locations.len());
            let end = rng.usize(start..=track.block_locations.len());
            let EventCount(count) = index.range_query(start..end);
            let correct: usize = track.block_locations[start..end]
                .iter()
                .map(|i| pool.blocks[*i as usize].len as usize)
                .sum();
            assert_eq!(count, correct, "failed for {}..{}", start, end);
        }
    }

    #[test]
    fn prop_test_aggregate_by_steps() {
        let mut pool = BlockPool::new();
        let mut track = Track::new();
        let rng = Rng::new();
        track.add_dummy_events(&mut pool, &rng, 325);

        let index = IForestIndex::<EventSum>::build(&track, &pool);

        let time_bounds = 0..=(track.end_time(&pool).unwrap() + 100_000);
        for _ in 0..100_000 {
            let t1 = rng.u64(time_bounds.clone());
            let t2 = rng.u64(time_bounds.clone());
            let t_range = if t2 > t1 { t1..t2 } else { t2..t1 };
            let range_size = t_range.end - t_range.start;
            let step = (range_size / rng.u64(1..10)) + rng.u64(0..100);

            let res1 = crate::aggregate_by_steps::<EventSum>(
                &pool,
                &track.block_locations,
                &index,
                t_range.clone(),
                step,
            );

            let res2 = crate::aggregate_by_steps_unindexed::<EventSum>(
                &pool,
                &track.block_locations,
                t_range.clone(),
                step,
            );

            assert_eq!(res1, res2, "failed for {:?} - {}", t_range, step);
        }
    }
}
