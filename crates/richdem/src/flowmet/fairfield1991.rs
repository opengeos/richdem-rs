//! Fairfield & Leymarie (1991) Rho8 and Rho4 stochastic flow proportions.
//!
//! Reference: Fairfield, J., Leymarie, P., 1991. Drainage networks from grid
//! digital elevation models. Water Resources Research 27, 709-717.

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::array3d::Array3D;
use crate::constants::{
    D4_NORTH, D4_SOUTH, HAS_FLOW_GEN, N8_DIAG, NO_DATA_GEN, NO_FLOW_GEN, Topology,
};
use crate::num::DemNum;
use crate::random::uniform_rand_real;

/// Computes Rho8 (`Topology::D8`) or Rho4 (`Topology::D4`) flow proportions.
pub fn fm_fairfield_leymarie<T: DemNum>(elevations: &Array2D<T>, topo: Topology) -> Array3D<f32> {
    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();
    let w = elevations.width();
    let h = elevations.height();

    let mut props = Array3D::<f32>::make_from_template(elevations, NO_FLOW_GEN);
    props.set_no_data(NO_DATA_GEN);

    for y in 0..h {
        for x in 0..w {
            if elevations.is_no_data(x, y) {
                props.set(x, y, 0, NO_DATA_GEN);
                continue;
            }
            if elevations.is_edge_cell(x, y) {
                continue;
            }
            let e = elevations.get(x, y);
            let mut greatest_n: usize = 0;
            let mut greatest_slope = 0.0f64;

            for n in 1..=nmax {
                let nx = x as i32 + dx[n];
                let ny = y as i32 + dy[n];
                if !elevations.in_grid(nx, ny) || elevations.is_no_data_xy(nx, ny) {
                    continue;
                }
                let ne = elevations.get_xy(nx, ny);
                if ne.rd_cmp(e) != Ordering::Less {
                    continue;
                }

                let mut rho_slope = e.to_f64() - ne.to_f64();
                if topo == Topology::D8 && N8_DIAG[n] {
                    rho_slope *= 1.0 / (2.0 - uniform_rand_real(0.0, 1.0));
                } else if topo == Topology::D4 && (n == D4_NORTH || n == D4_SOUTH) {
                    let r = uniform_rand_real(1e-12, 1.0);
                    rho_slope *= 1.0 / (1.0 / r - 1.0);
                }

                if rho_slope > greatest_slope {
                    greatest_n = n;
                    greatest_slope = rho_slope;
                }
            }

            if greatest_n == 0 {
                continue;
            }

            props.set(x, y, 0, HAS_FLOW_GEN);
            let d8_slot = match topo {
                Topology::D8 => greatest_n,
                Topology::D4 => match greatest_n {
                    1 => 1,
                    2 => 3,
                    3 => 5,
                    4 => 7,
                    _ => unreachable!(),
                },
            };
            props.set(x, y, d8_slot, 1.0);
        }
    }

    props
}

/// Computes Rho8 flow proportions.
pub fn fm_rho8<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    fm_fairfield_leymarie(elevations, Topology::D8)
}

/// Computes Rho4 flow proportions.
pub fn fm_rho4<T: DemNum>(elevations: &Array2D<T>) -> Array3D<f32> {
    fm_fairfield_leymarie(elevations, Topology::D4)
}
