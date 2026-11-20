//! Priority-Flood depression filling (Zhou et al., 2016 one-pass variant).
//!
//! Reference: Zhou, G., Sun, Z., Fu, S., 2016. An efficient variant of the
//! Priority-Flood algorithm for filling depressions in raster digital elevation models.
//! Computers & Geosciences 90, Part A, 87-96. doi:10.1016/j.cageo.2016.02.021

use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

use crate::array2d::Array2D;
use crate::constants::{D8X, D8Y};
use crate::num::DemNum;

#[derive(Debug, Clone, Copy)]
struct ZhouNode<T: DemNum> {
    elev: T,
    idx: usize,
}

impl<T: DemNum> PartialEq for ZhouNode<T> {
    fn eq(&self, other: &Self) -> bool {
        self.elev.rd_eq(other.elev) && self.idx == other.idx
    }
}

impl<T: DemNum> Eq for ZhouNode<T> {}

impl<T: DemNum> Ord for ZhouNode<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by elevation, then by index
        match other.elev.rd_cmp(self.elev) {
            Ordering::Equal => other.idx.cmp(&self.idx),
            ord => ord,
        }
    }
}

impl<T: DemNum> PartialOrd for ZhouNode<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const LABEL_UNVISITED: u8 = 0;
const LABEL_VISITED: u8 = 10;
const LABEL_NODATA: u8 = 255;

fn process_trace_que_onepass<T: DemNum>(
    dem: &Array2D<T>,
    labels: &mut Array2D<u8>,
    trace_queue: &mut VecDeque<usize>,
    priority_queue: &mut BinaryHeap<ZhouNode<T>>,
) {
    while let Some(c) = trace_queue.pop_front() {
        let mut in_pq = false;
        let ce = dem.get_i(c);
        let cl = labels.get_i(c);

        for n in 1..=8 {
            let Some(ni) = dem.n_to_i(c, D8X[n], D8Y[n]) else {
                continue;
            };
            if labels.get_i(ni) != LABEL_UNVISITED {
                continue;
            }

            let ne = dem.get_i(ni);
            if ce.rd_cmp(ne) == Ordering::Less {
                trace_queue.push_back(ni);
                labels.set_i(ni, cl);
                continue;
            }

            if !in_pq {
                let mut is_boundary = true;
                for nn in 1..=8 {
                    let Some(nni) = dem.n_to_i(ni, D8X[nn], D8Y[nn]) else {
                        continue;
                    };
                    if labels.get_i(nni) == LABEL_VISITED
                        && dem.get_i(nni).rd_cmp(ne) == Ordering::Less
                    {
                        is_boundary = false;
                        break;
                    }
                }
                if is_boundary {
                    priority_queue.push(ZhouNode { elev: ce, idx: c });
                    in_pq = true;
                }
            }
        }
    }
}

fn process_pit_onepass<T: DemNum>(
    c_elev: T,
    dem: &mut Array2D<T>,
    labels: &mut Array2D<u8>,
    depression_que: &mut VecDeque<usize>,
    trace_queue: &mut VecDeque<usize>,
) {
    while let Some(c) = depression_que.pop_front() {
        let cl = labels.get_i(c);
        for n in 1..=8 {
            let Some(ni) = dem.n_to_i(c, D8X[n], D8Y[n]) else {
                continue;
            };
            if labels.get_i(ni) != LABEL_UNVISITED {
                continue;
            }
            labels.set_i(ni, cl);
            if dem.get_i(ni).rd_cmp(c_elev) == Ordering::Greater {
                trace_queue.push_back(ni);
            } else {
                dem.set_i(ni, c_elev);
                depression_que.push_back(ni);
            }
        }
    }
}

/// Fills all D8 depressions in `dem` using Zhou et al. (2016) one-pass Priority-Flood.
pub fn priority_flood_zhou2016<T: DemNum>(dem: &mut Array2D<T>) {
    let w = dem.width();
    let h = dem.height();
    if w == 0 || h == 0 {
        return;
    }

    let mut trace_queue = VecDeque::<usize>::new();
    let mut depression_que = VecDeque::<usize>::new();
    let mut labels = Array2D::<u8>::new(w, h, LABEL_UNVISITED);
    let mut priority_queue = BinaryHeap::<ZhouNode<T>>::new();
    let nd = dem.no_data();

    // Mark all NoData cells so they are never modified or treated as terrain pits,
    // and seed valid perimeter cells plus valid cells adjacent to NoData.
    for y in 0..h {
        for x in 0..w {
            let i = dem.xy_to_i(x, y);
            let v = dem.get_i(i);
            if v.rd_eq(nd) {
                labels.set_i(i, LABEL_NODATA);
                continue;
            }
            if dem.is_edge_cell(x, y) {
                priority_queue.push(ZhouNode { elev: v, idx: i });
                labels.set_i(i, LABEL_VISITED);
                continue;
            }
            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if dem.in_grid(nx, ny) && dem.get_xy(nx, ny).rd_eq(nd) {
                    priority_queue.push(ZhouNode { elev: v, idx: i });
                    labels.set_i(i, LABEL_VISITED);
                    break;
                }
            }
        }
    }

    while let Some(c) = priority_queue.pop() {
        labels.set_i(c.idx, LABEL_VISITED);

        for n in 1..=8 {
            let Some(ni) = dem.n_to_i(c.idx, D8X[n], D8Y[n]) else {
                continue;
            };
            if labels.get_i(ni) != LABEL_UNVISITED {
                continue;
            }
            labels.set_i(ni, LABEL_VISITED);

            if dem.get_i(ni).rd_cmp(c.elev) != Ordering::Greater {
                dem.set_i(ni, c.elev);
                depression_que.push_back(ni);
                process_pit_onepass(
                    c.elev,
                    dem,
                    &mut labels,
                    &mut depression_que,
                    &mut trace_queue,
                );
            } else {
                trace_queue.push_back(ni);
            }
            process_trace_que_onepass(dem, &mut labels, &mut trace_queue, &mut priority_queue);
        }
    }
}
