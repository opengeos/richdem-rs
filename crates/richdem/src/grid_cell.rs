//! Grid cell coordinates and priority queues matching RichDEM's `grid_cell.hpp`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::num::DemNum;

/// Stores the `(x, y)` coordinates of a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GridCell {
    /// Column index.
    pub x: i32,
    /// Row index.
    pub y: i32,
}

impl GridCell {
    /// Creates a new grid cell at `(x, y)`.
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Stores `(x, y, z)` coordinates of a grid cell, ordered as a min-heap by `z`
/// (with NaN treated as infinitely small, matching RichDEM).
#[derive(Debug, Clone, Copy)]
pub struct GridCellZ<T: DemNum> {
    /// Column index.
    pub x: i32,
    /// Row index.
    pub y: i32,
    /// Elevation value.
    pub z: T,
}

impl<T: DemNum> GridCellZ<T> {
    /// Creates a new `(x, y, z)` grid cell.
    #[inline]
    pub const fn new(x: i32, y: i32, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T: DemNum> PartialEq for GridCellZ<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.z.rd_eq(other.z)
    }
}

impl<T: DemNum> Eq for GridCellZ<T> {}

impl<T: DemNum> Ord for GridCellZ<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed so Rust's max-heap BinaryHeap pops the smallest z first.
        other.z.rd_cmp(self.z)
    }
}

impl<T: DemNum> PartialOrd for GridCellZ<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Min-priority queue of [`GridCellZ`] ordered by ascending elevation.
pub type GridCellZPq<T> = BinaryHeap<GridCellZ<T>>;

/// Cell with insertion counter `k`; lower `k` (inserted earlier / FIFO) pops first on ties.
#[derive(Debug, Clone, Copy)]
pub struct GridCellZkLow<T: DemNum> {
    /// Column index.
    pub x: i32,
    /// Row index.
    pub y: i32,
    /// Elevation value.
    pub z: T,
    /// Insertion sequence number.
    pub k: u64,
}

impl<T: DemNum> PartialEq for GridCellZkLow<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.z.rd_eq(other.z) && self.k == other.k
    }
}

impl<T: DemNum> Eq for GridCellZkLow<T> {}

impl<T: DemNum> Ord for GridCellZkLow<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by z, then min-heap by k (earlier insertion first).
        match other.z.rd_cmp(self.z) {
            Ordering::Equal => other.k.cmp(&self.k),
            ord => ord,
        }
    }
}

impl<T: DemNum> PartialOrd for GridCellZkLow<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority queue sorted by ascending elevation; ties broken FIFO (earlier insertion first).
#[derive(Debug, Clone)]
pub struct GridCellZkLowPq<T: DemNum> {
    heap: BinaryHeap<GridCellZkLow<T>>,
    count: u64,
}

impl<T: DemNum> Default for GridCellZkLowPq<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DemNum> GridCellZkLowPq<T> {
    /// Creates an empty priority queue.
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            count: 0,
        }
    }

    /// Pushes `(x, y, z)` with the next monotonically increasing insertion counter.
    #[inline]
    pub fn emplace(&mut self, x: i32, y: i32, z: T) {
        self.count += 1;
        self.heap.push(GridCellZkLow {
            x,
            y,
            z,
            k: self.count,
        });
    }

    /// Peeks at the lowest element.
    #[inline]
    pub fn peek(&self) -> Option<&GridCellZkLow<T>> {
        self.heap.peek()
    }

    /// Pops the lowest element.
    #[inline]
    pub fn pop(&mut self) -> Option<GridCellZkLow<T>> {
        self.heap.pop()
    }

    /// Returns `true` if the queue is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Number of elements in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

/// Cell with insertion counter `k`; higher `k` (inserted later / LIFO) pops first on ties.
#[derive(Debug, Clone, Copy)]
pub struct GridCellZkHigh<T: DemNum> {
    /// Column index.
    pub x: i32,
    /// Row index.
    pub y: i32,
    /// Elevation value.
    pub z: T,
    /// Insertion sequence number.
    pub k: u64,
}

impl<T: DemNum> PartialEq for GridCellZkHigh<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.z.rd_eq(other.z) && self.k == other.k
    }
}

impl<T: DemNum> Eq for GridCellZkHigh<T> {}

impl<T: DemNum> Ord for GridCellZkHigh<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by z, then max-heap by k (later insertion first).
        match other.z.rd_cmp(self.z) {
            Ordering::Equal => self.k.cmp(&other.k),
            ord => ord,
        }
    }
}

impl<T: DemNum> PartialOrd for GridCellZkHigh<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Priority queue sorted by ascending elevation; ties broken LIFO (later insertion first).
#[derive(Debug, Clone)]
pub struct GridCellZkHighPq<T: DemNum> {
    heap: BinaryHeap<GridCellZkHigh<T>>,
    count: u64,
}

impl<T: DemNum> Default for GridCellZkHighPq<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DemNum> GridCellZkHighPq<T> {
    /// Creates an empty priority queue.
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            count: 0,
        }
    }

    /// Pushes `(x, y, z)` with the next monotonically increasing insertion counter.
    #[inline]
    pub fn emplace(&mut self, x: i32, y: i32, z: T) {
        self.count += 1;
        self.heap.push(GridCellZkHigh {
            x,
            y,
            z,
            k: self.count,
        });
    }

    /// Peeks at the lowest element.
    #[inline]
    pub fn peek(&self) -> Option<&GridCellZkHigh<T>> {
        self.heap.peek()
    }

    /// Pops the lowest element.
    #[inline]
    pub fn pop(&mut self) -> Option<GridCellZkHigh<T>> {
        self.heap.pop()
    }

    /// Returns `true` if the queue is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Number of elements in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

/// Key-value min-heap entry ordered by `key` ascending (and `k` LIFO on ties if needed).
#[derive(Debug, Clone, Copy)]
pub struct KeyValEntry<K: DemNum, V: Copy> {
    /// Priority key (elevation).
    pub key: K,
    /// Payload value (flat cell index).
    pub value: V,
    /// Sequence number for deterministic tie-breaking.
    pub seq: u64,
}

impl<K: DemNum, V: Copy> PartialEq for KeyValEntry<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key.rd_eq(other.key) && self.seq == other.seq
    }
}

impl<K: DemNum, V: Copy> Eq for KeyValEntry<K, V> {}

impl<K: DemNum, V: Copy> Ord for KeyValEntry<K, V> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by key; LIFO by seq on ties (matching DepressionHierarchy's
        // requirement that recently added flat cells pop first).
        match other.key.rd_cmp(self.key) {
            Ordering::Equal => self.seq.cmp(&other.seq),
            ord => ord,
        }
    }
}

impl<K: DemNum, V: Copy> PartialOrd for KeyValEntry<K, V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
