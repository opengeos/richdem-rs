//! Miscellaneous raster methods: surface area (Jenness 2004), perimeter,
//! bucket fill, and flow loop checking.

use crate::array2d::{Array2D, map_range_ordered};
use crate::constants::{D8X, D8Y, N8_DIAG, NO_FLOW, Topology};
use crate::error::{Error, Result};
use crate::num::DemNum;

/// Calculates topographic surface area of a DEM using Jenness (2004).
///
/// Accumulates deterministic per-row partial sums in row order so parallel and
/// serial builds produce identical results across thread counts.
///
/// Reference: Jenness, J.S., 2004. Calculating landscape surface area from digital
/// elevation models. Wildlife Society Bulletin 32, 829-839.
pub fn dem_surface_area<T: DemNum>(elevations: &Array2D<T>, zscale: f64) -> Result<f64> {
    const FUDGE_FACTOR: f64 = 1e-4;
    let xdist = elevations.get_cell_length_x();
    let ydist = elevations.get_cell_length_y();
    let planar_diag_dist = xdist.hypot(ydist);
    let cell_planar_area = elevations.get_cell_area();
    let w = elevations.width();
    let h = elevations.height();

    let row_area = |y: usize| -> Result<(f64, usize)> {
        let mut r_area = 0.0f64;
        let mut r_valid = 0usize;
        for x in 0..w {
            if elevations.is_no_data(x, y) {
                continue;
            }
            let my_elev = zscale * elevations.get(x, y).to_f64();
            if !my_elev.is_finite() {
                continue;
            }
            r_valid += 1;
            let mut cell_area = 0.0f64;

            for (n, &is_diag) in N8_DIAG.iter().enumerate().skip(1) {
                let nn = if n == 8 { 1 } else { n + 1 };
                let (dn, ndn) = if is_diag { (n, nn) } else { (nn, n) };

                let dnx = x as i32 + D8X[dn];
                let dny = y as i32 + D8Y[dn];
                let dn_elev = if elevations.in_grid(dnx, dny) && !elevations.is_no_data_xy(dnx, dny)
                {
                    let v = zscale * elevations.get_xy(dnx, dny).to_f64();
                    if v.is_finite() { v } else { my_elev }
                } else {
                    my_elev
                };

                let ndnx = x as i32 + D8X[ndn];
                let ndny = y as i32 + D8Y[ndn];
                let ndn_elev =
                    if elevations.in_grid(ndnx, ndny) && !elevations.is_no_data_xy(ndnx, ndny) {
                        let v = zscale * elevations.get_xy(ndnx, ndny).to_f64();
                        if v.is_finite() { v } else { my_elev }
                    } else {
                        my_elev
                    };

                let planar_dist_dn = planar_diag_dist;
                let planar_dist_ndn = if D8Y[ndn] == 0 { xdist } else { ydist };
                let planar_dist_bn = if D8Y[ndn] == 0 { ydist } else { xdist };

                let elev_diff_dn = dn_elev - my_elev;
                let elev_diff_ndn = ndn_elev - my_elev;
                let elev_diff_bn = ndn_elev - dn_elev;

                let surf_dist_dn = planar_dist_dn.hypot(elev_diff_dn) / 2.0;
                let surf_dist_ndn = planar_dist_ndn.hypot(elev_diff_ndn) / 2.0;
                let surf_dist_bn = planar_dist_bn.hypot(elev_diff_bn) / 2.0;

                let s = (surf_dist_dn + surf_dist_ndn + surf_dist_bn) / 2.0;
                let heron = (s * (s - surf_dist_dn) * (s - surf_dist_ndn) * (s - surf_dist_bn))
                    .max(0.0)
                    .sqrt();
                cell_area += heron;
            }

            if cell_area < cell_planar_area {
                if cell_area + FUDGE_FACTOR >= cell_planar_area {
                    cell_area = cell_planar_area;
                } else {
                    return Err(Error::Algorithm(
                        "A cell had a topographic surface area less than its planar surface area!"
                            .into(),
                    ));
                }
            }
            r_area += cell_area;
        }
        Ok((r_area, r_valid))
    };

    let mut area = 0.0f64;
    let mut valid_cells = 0usize;
    for row in map_range_ordered(h, row_area) {
        let (r_area, r_valid) = row?;
        area += r_area;
        valid_cells += r_valid;
    }

    let dem_planar_area = (valid_cells as f64) * cell_planar_area;
    Ok(area.max(dem_planar_area))
}

/// Method used by [`perimeter`] to measure the boundary of valid data cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PerimType {
    /// Count of cells bordering DEM edges.
    CellCount,
    /// Sum of cell edge lengths bordering DEM edges or NoData cells.
    SquareEdge,
}

/// Calculates the perimeter of a raster under [`PerimType`].
pub fn perimeter<T: DemNum>(arr: &Array2D<T>, perim_type: PerimType) -> f64 {
    let w = arr.width();
    let h = arr.height();

    let row_counts = |y: usize| -> (u64, u64, u64) {
        let mut vertical_edges: u64 = 0;
        let mut horizontal_edges: u64 = 0;
        let mut cell_edges: u64 = 0;
        for x in 0..w {
            if arr.is_no_data(x, y) {
                continue;
            }
            match perim_type {
                PerimType::CellCount => {
                    for n in 1..=8 {
                        if !arr.in_grid(x as i32 + D8X[n], y as i32 + D8Y[n]) {
                            cell_edges += 1;
                            break;
                        }
                    }
                }
                PerimType::SquareEdge => {
                    for n in 1..=8 {
                        let nx = x as i32 + D8X[n];
                        let ny = y as i32 + D8Y[n];
                        if !arr.in_grid(nx, ny) || arr.is_no_data_xy(nx, ny) {
                            if D8X[n] == 0 {
                                horizontal_edges += 1;
                            } else if D8Y[n] == 0 {
                                vertical_edges += 1;
                            }
                        }
                    }
                }
            }
        }
        (vertical_edges, horizontal_edges, cell_edges)
    };

    let (vertical_edges, horizontal_edges, cell_edges) = map_range_ordered(h, row_counts)
        .into_iter()
        .fold((0u64, 0u64, 0u64), |(v1, h1, c1), (v2, h2, c2)| {
            (v1 + v2, h1 + h2, c1 + c2)
        });

    match perim_type {
        PerimType::CellCount => cell_edges as f64,
        PerimType::SquareEdge => {
            (horizontal_edges as f64) * arr.get_cell_length_x()
                + (vertical_edges as f64) * arr.get_cell_length_y()
        }
    }
}

/// Flood-fills `set_raster` with `set_value` starting from `seeds` wherever `check_raster`
/// equals `check_value`.
pub fn bucket_fill<T: DemNum, U: DemNum>(
    check_raster: &Array2D<T>,
    set_raster: &mut Array2D<U>,
    check_value: T,
    set_value: U,
    seeds: &mut Vec<usize>,
    topo: Topology,
) -> Result<()> {
    if check_raster.width() != set_raster.width() || check_raster.height() != set_raster.height() {
        return Err(Error::DimensionMismatch(
            "Rasters must have the same dimension for bucket_fill!".into(),
        ));
    }

    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();
    let mut visited = vec![false; check_raster.size()];

    while let Some(c) = seeds.pop() {
        if c >= visited.len() || visited[c] || !check_raster.get_i(c).rd_eq(check_value) {
            continue;
        }
        visited[c] = true;
        set_raster.set_i(c, set_value);
        let (cx, cy) = check_raster.i_to_xy(c);
        for n in 1..=nmax {
            let nx = cx as i32 + dx[n];
            let ny = cy as i32 + dy[n];
            if !check_raster.in_grid(nx, ny) {
                continue;
            }
            let ni = check_raster.xy_to_i(nx as usize, ny as usize);
            if !visited[ni] && check_raster.get_i(ni).rd_eq(check_value) {
                seeds.push(ni);
            }
        }
    }

    Ok(())
}

/// Runs [`bucket_fill`] seeded from all perimeter edges of `check_raster`.
pub fn bucket_fill_from_edges<T: DemNum, U: DemNum>(
    check_raster: &Array2D<T>,
    set_raster: &mut Array2D<U>,
    check_value: T,
    set_value: U,
    topo: Topology,
) -> Result<()> {
    let w = check_raster.width();
    let h = check_raster.height();
    let mut seeds = Vec::with_capacity(2 * w + 2 * h);
    if w > 0 && h > 0 {
        for y in 0..h {
            seeds.push(check_raster.xy_to_i(0, y));
            seeds.push(check_raster.xy_to_i(w - 1, y));
        }
        for x in 0..w {
            seeds.push(check_raster.xy_to_i(x, 0));
            seeds.push(check_raster.xy_to_i(x, h - 1));
        }
    }
    bucket_fill(
        check_raster,
        set_raster,
        check_value,
        set_value,
        &mut seeds,
        topo,
    )
}

/// Checks a D8 flow direction raster for cycles/loops.
/// Returns a list of `(x, y)` coordinates where flow cycles were detected.
pub fn find_flow_loops<T: DemNum>(flowdirs: &Array2D<T>) -> Vec<(usize, usize)> {
    let n = flowdirs.size();
    // 0 = unvisited, 1 = visiting (on current stack), 2 = finished (acyclic)
    let mut state = vec![0u8; n];
    let mut loops = Vec::new();
    let mut path = Vec::new();

    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        path.clear();
        let mut cur = start;
        loop {
            if state[cur] == 2 {
                break;
            }
            if state[cur] == 1 {
                loops.push(flowdirs.i_to_xy(cur));
                break;
            }
            state[cur] = 1;
            path.push(cur);

            if flowdirs.is_no_data_i(cur) {
                break;
            }
            let dir = flowdirs.get_i(cur).to_i64();
            if dir == NO_FLOW as i64 || !(1..=8).contains(&dir) {
                break;
            }
            let Some(next) = flowdirs.n_to_i(cur, D8X[dir as usize], D8Y[dir as usize]) else {
                break;
            };
            cur = next;
        }
        for &idx in &path {
            state[idx] = 2;
        }
    }

    loops
}
