use crate::{
    index::{Aggregate, TrackIndex},
    trace::{BlockPool, TraceBlock, Track},
};
use core::{marker::PhantomData, ops::Range};

#[derive(Debug)]
#[repr(transparent)]
pub struct IForestIndex<K, A> {
    pub values: Vec<A>,
    __kind: PhantomData<K>,
}

impl<K, A> IForestIndex<K, A> {
    pub const fn new() -> Self {
        IForestIndex {
            values: Vec::new(),
            __kind: PhantomData,
        }
    }
}

//                #
// _______________|
// _______|_______|   #
// ___|___|___|___|___|
// 0|1|2|3|4|5|6|7|8|9|
impl<K, A> IForestIndex<K, A>
where
    A: Aggregate<K> + Clone,
{
    pub fn push(&mut self, block: &TraceBlock<K>)
    where
        K: Default,
    {
        self.values.push(A::from_block(block));

        let len = self.values.len();
        // We want to index the first level every 2 nodes, 2nd level every 4 nodes...
        // This happens to correspond to the number of trailing ones in the index
        let levels_to_index = len.trailing_ones() - 1;

        // Complete unfinished aggregation nodes which are now ready
        let mut cur = len - 1; // The leaf we just pushed
        for level in 0..levels_to_index {
            let prev_higher_level = cur - (1 << level); // nodes at a level reach 2^level
            let combined = A::join(&self.values[prev_higher_level], &self.values[cur]);
            self.values[prev_higher_level] = combined;
            cur = prev_higher_level;
        }

        // Push new aggregation node going back one level further than we aggregated
        self.values
            .push(self.values[len - (1 << levels_to_index)].clone());
    }

    pub fn range_query(&self, queried_range: Range<usize>) -> A {
        const fn left_child_at(node: usize, level: usize) -> bool {
            // every even power of two block at each level is on the left
            (node >> level) & 1 == 0
        }

        const fn skip(level: usize) -> usize {
            // lvl 0 skips self and agg node next to it, steps up by powers of 2
            2 << level
        }

        const fn agg_node(node: usize, level: usize) -> usize {
            // lvl 0 is us+0, lvl 1 is us+1, steps by power of 2
            node + (1 << level) - 1
        }

        let mut range = (queried_range.start * 2)..(queried_range.end * 2); // translate underlying to interior indices
        let len = self.values.len();
        debug_assert!(
            range.start <= len && range.end <= len,
            "range {:?} not inside 0..{}",
            queried_range,
            len / 2,
        );

        let mut combined = A::empty();
        while range.start < range.end {
            // Skip via the highest level where we're on the very left and it isn't too far
            let mut up_level = 1;
            while left_child_at(range.start, up_level) && range.start + skip(up_level) <= range.end
            {
                up_level += 1;
            }

            let level = up_level - 1;
            combined = A::join(&combined, &self.values[agg_node(range.start, level)]);

            range.start += skip(level);
        }

        combined
    }
}

impl<K, A> Default for IForestIndex<K, A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, A> TrackIndex<K, A> for IForestIndex<K, A>
where
    K: Default,
    A: Aggregate<K> + Clone,
{
    fn build(track: &Track<K>, pool: &BlockPool<K>) -> IForestIndex<K, A> {
        let mut forest = IForestIndex::new();
        for &idx in &track.block_locations {
            forest.push(&pool.blocks[idx as usize]);
        }

        // TODO in parallel
        forest
    }
}
