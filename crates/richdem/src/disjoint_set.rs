//! Disjoint-set (union-find) data structure for dense integer sets.
//!
//! Ported from RichDEM's `include/richdem/common/disjoint_dense_int_set.hpp`.

/// A disjoint-set / union-find structure over dense integer IDs `0..N`.
#[derive(Debug, Clone, Default)]
pub struct DisjointDenseIntSet {
    rank: Vec<u32>,
    parent: Vec<u32>,
}

impl DisjointDenseIntSet {
    /// Creates an empty disjoint set. Sets can be created dynamically via [`Self::make_set`]
    /// or [`Self::merge_a_into_b`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a disjoint set preallocated with `n` singleton sets `0..n`.
    pub fn with_size(n: usize) -> Self {
        let mut parent = Vec::with_capacity(n);
        for i in 0..n {
            parent.push(i as u32);
        }
        Self {
            rank: vec![0; n],
            parent,
        }
    }

    fn check_size(&mut self, new_n: u32) {
        let target = new_n as usize;
        if target < self.rank.len() {
            return;
        }
        let old_size = self.rank.len();
        self.rank.resize(target + 1, 0);
        self.parent.resize(target + 1, 0);
        for (i, slot) in self
            .parent
            .iter_mut()
            .enumerate()
            .take(target + 1)
            .skip(old_size)
        {
            *slot = i as u32;
        }
    }

    /// Explicitly ensures set `n` (and all sets `0..=n`) exist.
    pub fn make_set(&mut self, n: u32) {
        self.check_size(n);
    }

    /// Highest set ID currently allocated.
    pub fn max_element(&self) -> Option<u32> {
        self.rank.len().checked_sub(1).map(|v| v as u32)
    }

    /// Finds the representative set ID of `n` using path compression.
    pub fn find_set(&mut self, n: u32) -> u32 {
        let idx = n as usize;
        assert!(
            idx < self.parent.len(),
            "DisjointDenseIntSet::find_set({n}) outside valid range [0, {})",
            self.parent.len()
        );
        // Iterative path compression to prevent stack overflow on deep chains
        let mut root = n;
        while self.parent[root as usize] != root {
            root = self.parent[root as usize];
        }
        let mut cur = n;
        while self.parent[cur as usize] != root {
            let next = self.parent[cur as usize];
            self.parent[cur as usize] = root;
            cur = next;
        }
        root
    }

    /// Joins the sets containing `a` and `b` using union-by-rank.
    pub fn union_set(&mut self, a: u32, b: u32) {
        let mut roota = self.find_set(a);
        let mut rootb = self.find_set(b);
        if roota == rootb {
            return;
        }
        if self.rank[roota as usize] > self.rank[rootb as usize] {
            std::mem::swap(&mut roota, &mut rootb);
        }
        self.parent[roota as usize] = rootb;
        if self.rank[roota as usize] == self.rank[rootb as usize] {
            self.rank[rootb as usize] += 1;
        }
    }

    /// Merges the set containing `a` into the set containing `b`, preserving
    /// `b`'s root representative as the combined root.
    ///
    /// Both `a` and `b` are resolved to their current representatives first, so
    /// calling `merge_a_into_b` on non-roots or already-merged elements cannot
    /// introduce parent cycles.
    pub fn merge_a_into_b(&mut self, a: u32, b: u32) {
        self.check_size(a);
        self.check_size(b);
        let roota = self.find_set(a);
        let rootb = self.find_set(b);
        if roota == rootb {
            return;
        }
        self.parent[roota as usize] = rootb;
        let ra = self.rank[roota as usize];
        let rb = self.rank[rootb as usize];
        if ra == rb {
            self.rank[rootb as usize] += 1;
        } else if ra > rb {
            self.rank[rootb as usize] = ra + 1;
        }
    }

    /// Returns `true` if `a` and `b` belong to the same set.
    pub fn same_set(&mut self, a: u32, b: u32) -> bool {
        self.find_set(a) == self.find_set(b)
    }
}
