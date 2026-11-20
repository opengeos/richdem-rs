//! O'Callaghan & Mark (1984) D8 and D4 flow proportions.
//!
//! Reference: O'Callaghan, J.F., Mark, D.M., 1984. The Extraction of Drainage
//! Networks from Digital Elevation Data. Computer Vision, Graphics, and Image
//! Processing 28, 323-344.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::array3d::{Array3D, FLOW_PROPS_DEPTH};
use crate::constants::{HAS_FLOW_GEN, NO_DATA_GEN, NO_FLOW_GEN, Topology};
use crate::num::DemNum;

/// Computes D8 or D4 flow proportions into `Array3D<f32>`.
pub fn fm_ocallaghan<T: DemNum>(elevations: &Array2D<T>, topo: Topology) -> Array3D<f32> {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();
    let w = elevations.width();
    let h = elevations.height();

    let mut props = Array3D::<f32>::make_from_template(elevations, NO_FLOW_GEN);
    props.set_no_data(NO_DATA_GEN);
    if w == 0 || h == 0 {
        return props;
    }

    let row_stride = w * FLOW_PROPS_DEPTH;
    let process_row = |y: usize, row: &mut [f32]| {
        for x in 0..w {
            let cell = &mut row[x * FLOW_PROPS_DEPTH..(x + 1) * FLOW_PROPS_DEPTH];
            if elevations.is_no_data(x, y) {
                cell[0] = NO_DATA_GEN;
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                continue;
            }
            let ce = elevations.get(x, y);
            let mut lowest_n: usize = 0;
            let mut lowest_n_elev = T::MAX;

            for n in 1..=nmax {
                let nx = x as i32 + dx[n];
                let ny = y as i32 + dy[n];
                if elevations.is_no_data_xy(nx, ny) {
                    continue;
                }
                let ne = elevations.get_xy(nx, ny);
                if ne.rd_cmp(ce) != Ordering::Less {
                    continue;
                }
                if ne.rd_cmp(lowest_n_elev) == Ordering::Less {
                    lowest_n_elev = ne;
                    lowest_n = n;
                }
            }

            if lowest_n == 0 {
                continue;
            }
            cell[0] = HAS_FLOW_GEN;
            // Note: for D4, map D4 neighbor index to D8 slot so FlowAccumulation
            // (which iterates D8 offsets 1..=8) routes to the exact same cell!
            let d8_slot = match topo {
                Topology::D8 => lowest_n,
                Topology::D4 => match lowest_n {
                    1 => 1, // West
                    2 => 3, // North
                    3 => 5, // East
                    4 => 7, // South
                    _ => unreachable!(),
                },
            };
            cell[d8_slot] = 1.0;
        }
    };

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;
        props
            .data_mut()
            .par_chunks_mut(row_stride)
            .enumerate()
            .for_each(|(y, row)| process_row(y, row));
    }

    #[cfg(not(feature = "parallel"))]
    {
        for (y, row) in props.data_mut().chunks_mut(row_stride).enumerate() {
            process_row(y, row);
        }
    }

    props
}

/// Computes D8 flow proportions (alias for `fm_ocallaghan(elevations, Topology::D8)`).
pub fn fm_d8<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    fm_ocallaghan(elevations, Topology::D8)
}

/// Computes D4 flow proportions (alias for `fm_ocallaghan(elevations, Topology::D4)`).
pub fn fm_d4<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    fm_ocallaghan(elevations, Topology::D4)
}
