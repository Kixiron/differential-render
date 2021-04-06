use core::{
    cmp::Ordering,
    fmt::{self, Debug, Display},
    hash::{Hash, Hasher},
    marker::PhantomData,
};
use tinyvec::ArrayVec;

pub type Nanos = u64;

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct PackedNanos([u8; 6]);

impl PackedNanos {
    pub const fn new(ts: Nanos) -> Self {
        let b = ts.to_le_bytes();
        // assert!(b[6] == 0 && b[7] == 0);
        PackedNanos([b[0], b[1], b[2], b[3], b[4], b[5]])
    }

    #[allow(clippy::many_single_char_names)]
    pub const fn unpack(self) -> Nanos {
        let [a, b, c, d, e, f] = self.0;
        u64::from_le_bytes([a, b, c, d, e, f, 0, 0])
    }
}

impl Debug for PackedNanos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.unpack(), f)
    }
}

impl Display for PackedNanos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.unpack(), f)
    }
}

impl PartialEq for PackedNanos {
    fn eq(&self, other: &Self) -> bool {
        self.unpack().eq(&other.unpack())
    }
}

impl Eq for PackedNanos {}

impl PartialOrd for PackedNanos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.unpack().partial_cmp(&other.unpack())
    }
}

impl Ord for PackedNanos {
    fn cmp(&self, other: &Self) -> Ordering {
        self.unpack().cmp(&other.unpack())
    }
}

impl Hash for PackedNanos {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.unpack());
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceEvent<K> {
    pub kind: K,
    pub timestamp: PackedNanos,
    pub duration: PackedNanos,
}

pub type BlockIndex = u32;

const EVENTS_PER_BLOCK: usize = 16;

#[derive(Debug)]
#[repr(transparent)]
pub struct TraceBlock<K>
where
    K: Default,
{
    events: ArrayVec<[TraceEvent<K>; EVENTS_PER_BLOCK]>,
}

impl<K> TraceBlock<K>
where
    K: Default,
{
    pub fn new() -> Self {
        Self {
            events: ArrayVec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn is_full(&self) -> bool {
        !self.is_empty()
    }

    pub fn push(&mut self, ev: TraceEvent<K>) {
        assert!(!self.is_full());
        self.events.push(ev);
    }

    pub fn events(&self) -> &[TraceEvent<K>] {
        &self.events
    }

    /// Returns 0 if block is empty, `Track` has a useful invariant that
    /// blocks are never empty.
    pub fn start_time(&self) -> Nanos {
        self.events[0].timestamp.unpack()
    }
}

impl<K> Default for TraceBlock<K>
where
    K: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct BlockPool<K>
where
    K: Default,
{
    pub blocks: Vec<TraceBlock<K>>,
}

impl<K> BlockPool<K>
where
    K: Default,
{
    pub fn new() -> Self {
        BlockPool { blocks: Vec::new() }
    }

    pub fn alloc(&mut self) -> BlockIndex {
        let i = self.blocks.len();
        self.blocks.push(TraceBlock::new());
        i as BlockIndex
    }
}

impl<K> Default for BlockPool<K>
where
    K: Default,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Track<K> {
    pub block_locations: Vec<BlockIndex>,
    __kind: PhantomData<K>,
}

impl<K> Track<K> {
    pub fn new() -> Self {
        Self {
            block_locations: Vec::new(),
            __kind: PhantomData,
        }
    }
}

impl<K> Track<K>
where
    K: Default,
{
    fn new_block(&mut self, pool: &mut BlockPool<K>) -> BlockIndex {
        let i = pool.alloc();
        self.block_locations.push(i);
        i
    }

    pub fn push(&mut self, pool: &mut BlockPool<K>, ev: TraceEvent<K>) {
        let last = match self.block_locations.last() {
            None => self.new_block(pool),
            Some(&i) if pool.blocks[i as usize].is_full() => self.new_block(pool),
            Some(&i) => i,
        };

        pool.blocks[last as usize].push(ev)
    }

    pub fn start_time(&self, pool: &BlockPool<K>) -> Option<Nanos> {
        self.block_locations
            .get(0)
            .map(|i| pool.blocks[*i as usize].start_time())
    }

    pub fn end_time(&self, pool: &BlockPool<K>) -> Option<Nanos> {
        self.block_locations
            .last()
            .and_then(|i| pool.blocks[*i as usize].events().last())
            .map(|x| x.timestamp.unpack())
    }

    pub fn after_last_time(&self, pool: &BlockPool<K>) -> Option<Nanos> {
        self.block_locations
            .last()
            .and_then(|i| pool.blocks[*i as usize].events().last())
            .map(|x| x.timestamp.unpack() + x.duration.unpack())
    }

    pub fn events<'a>(
        &'a self,
        pool: &'a BlockPool<K>,
    ) -> impl Iterator<Item = &'a TraceEvent<K>> + 'a {
        self.block_locations
            .iter()
            .flat_map(move |i| pool.blocks[*i as usize].events())
    }
}

impl<K> Default for Track<K>
where
    K: Default,
{
    fn default() -> Self {
        Self::new()
    }
}
