//! Depression breaching algorithms (Lindsay, 2016).
//!
//! Reference: Lindsay, J.B., 2016. Efficient hybrid breaching-filling sink removal
//! methods for flow path enforcement in digital elevation models. Hydrological
//! Processes 30, 846-857. doi:10.1002/hyp.10648

use std::cmp::Ordering;

use crate::array2d::Array2D;
use crate::constants::{D8X, D8Y, Topology};
use crate::error::{Error, Result};
use crate::grid_cell::GridCellZkLowPq;
use crate::num::DemNum;

/// Breaching mode for [`lindsay2016`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LindsayMode {
    /// All depressions are completely breached.
    CompleteBreaching,
    /// Depressions are breached only if the path meets both length and depth limits.
    SelectiveBreaching,
    /// A breaching path is drilled up to the permitted length and depth limits.
    ConstrainedBreaching,
}

const UNVISITED: u8 = 0;
const VISITED: u8 = 1;
const EDGE: u8 = 2;
const NO_BACK_LINK: u32 = u32::MAX;

/// Complete depression breaching (Lindsay, 2016) under `topo` (D8 or D4).
pub fn complete_breaching_lindsay2016<T: DemNum>(dem: &mut Array2D<T>, topo: Topology) {
    let w = dem.width();
    let h = dem.height();
    if w == 0 || h == 0 {
        return;
    }

    let dx = topo.dx();
    let dy = topo.dy();
    let nmax = topo.nmax();

    let mut backlinks = vec![NO_BACK_LINK; w * h];
    let mut visited = vec![UNVISITED; w * h];
    let mut pits = vec![0u8; w * h];
    let mut pq = GridCellZkLowPq::<T>::new();
    let mut total_pits: u32 = 0;

    let nd = dem.no_data();
    let dem_data = dem.data_mut();

    for y in 0..h {
        let row_base = y * w;
        let is_y_edge = y == 0 || y + 1 == h;
        for x in 0..w {
            let ci = row_base + x;
            let ce = dem_data[ci];
            if ce.rd_eq(nd) {
                continue;
            }
            if is_y_edge || x == 0 || x + 1 == w {
                pq.emplace(x as i32, y as i32, ce);
                visited[ci] = EDGE;
                continue;
            }

            let mut lowest_neighbour = T::MAX;
            let mut borders_nodata = false;
            for n in 1..=nmax {
                let nx = (x as i32 + dx[n]) as usize;
                let ny = (y as i32 + dy[n]) as usize;
                let ne = dem_data[ny * w + nx];
                if ne.rd_eq(nd) {
                    pq.emplace(x as i32, y as i32, ce);
                    visited[ci] = EDGE;
                    borders_nodata = true;
                    break;
                }
                lowest_neighbour = lowest_neighbour.rd_min(ne);
            }
            if borders_nodata {
                continue;
            }

            if ce.rd_cmp(lowest_neighbour) != Ordering::Greater {
                dem_data[ci] = lowest_neighbour;
                pits[ci] = 1;
                total_pits += 1;
            }
        }
    }

    let wi32 = w as i32;
    let hi32 = h as i32;
    while let Some(c) = pq.pop() {
        let ci_usize = (c.y as usize) * w + (c.x as usize);
        if pits[ci_usize] != 0 {
            let mut cc = ci_usize as u32;
            let target_height = dem_data[ci_usize];

            while cc != NO_BACK_LINK
                && dem_data[cc as usize].rd_cmp(target_height) != Ordering::Less
            {
                dem_data[cc as usize] = target_height;
                cc = backlinks[cc as usize];
            }

            total_pits -= 1;
            if total_pits == 0 {
                break;
            }
        }

        let ci = ci_usize as u32;
        for n in 1..=nmax {
            let nx = c.x + dx[n];
            let ny = c.y + dy[n];
            if nx < 0 || ny < 0 || nx >= wi32 || ny >= hi32 {
                continue;
            }
            let ni = (ny as usize) * w + (nx as usize);
            if visited[ni] != UNVISITED {
                continue;
            }
            let my_e = dem_data[ni];
            if my_e.rd_eq(nd) {
                continue;
            }
            pq.emplace(nx, ny, my_e);
            visited[ni] = VISITED;
            backlinks[ni] = ci;
        }
    }
}

/// Hybrid depression breaching and filling (Lindsay, 2016) supporting Complete,
/// Selective, and Constrained breaching modes (D8 topology).
pub fn lindsay2016<T: DemNum>(
    dem: &mut Array2D<T>,
    mode: LindsayMode,
    eps_gradients: bool,
    fill_depressions: bool,
    max_path_len: u32,
    max_depth: T,
) -> Result<()> {
    if eps_gradients && !T::IS_FLOAT {
        return Err(Error::UnsupportedDataType(
            "Lindsay2016 eps_gradients requires a floating-point DEM".into(),
        ));
    }

    let w = dem.width();
    let h = dem.height();
    if w == 0 || h == 0 {
        return Ok(());
    }

    let orig_dem = if mode != LindsayMode::CompleteBreaching {
        Some(dem.clone())
    } else {
        None
    };

    let mut backlinks = Array2D::<u32>::new(w, h, NO_BACK_LINK);
    let mut visited = Array2D::<u8>::new(w, h, UNVISITED);
    let mut pits = Array2D::<u8>::new(w, h, 0);
    let mut flood_array: Vec<u32> = Vec::new();
    let mut pq = GridCellZkLowPq::<T>::new();
    let mut total_pits: u32 = 0;

    for y in 0..h {
        for x in 0..w {
            if dem.is_no_data(x, y) {
                continue;
            }
            if dem.is_edge_cell(x, y) {
                pq.emplace(x as i32, y as i32, dem.get(x, y));
                visited.set(x, y, EDGE);
                continue;
            }

            let mut lowest_neighbour = T::MAX;
            let mut borders_nodata = false;
            for n in 1..=8 {
                let nx = x as i32 + D8X[n];
                let ny = y as i32 + D8Y[n];
                if dem.is_no_data_xy(nx, ny) {
                    pq.emplace(x as i32, y as i32, dem.get(x, y));
                    visited.set(x, y, EDGE);
                    borders_nodata = true;
                    break;
                }
                lowest_neighbour = lowest_neighbour.rd_min(dem.get_xy(nx, ny));
            }
            if borders_nodata {
                continue;
            }

            if dem.get(x, y).rd_cmp(lowest_neighbour) == Ordering::Less {
                if eps_gradients {
                    dem.set(x, y, lowest_neighbour.next_down_float().unwrap());
                } else {
                    dem.set(x, y, lowest_neighbour);
                }
            }

            if dem.get(x, y).rd_cmp(lowest_neighbour) != Ordering::Greater {
                pits.set(x, y, 1);
                total_pits += 1;
            }
        }
    }

    let max_depth_f64 = max_depth.to_f64();
    let max_depth_i128 = max_depth.to_i128();

    while let Some(c) = pq.pop() {
        if pits.get_xy(c.x, c.y) != 0 {
            let mut pathlen: u32 = 0;
            let pit_idx = dem.xy_to_i(c.x as usize, c.y as usize) as u32;
            let mut cc = pit_idx;
            let mut pathdepth_f64 = f64::NEG_INFINITY;
            let mut pathdepth_i128 = i128::MIN;
            let mut target_height = dem.get_xy(c.x, c.y);

            if mode == LindsayMode::CompleteBreaching {
                while cc != NO_BACK_LINK
                    && dem.get_i(cc as usize).rd_cmp(target_height) != Ordering::Less
                {
                    dem.set_i(cc as usize, target_height);
                    cc = backlinks.get_i(cc as usize);
                    if eps_gradients {
                        target_height = target_height.next_down_float().unwrap();
                    }
                }
            } else {
                while cc != NO_BACK_LINK
                    && dem.get_i(cc as usize).rd_cmp(target_height) != Ordering::Less
                {
                    let ref_elev = orig_dem
                        .as_ref()
                        .map_or_else(|| dem.get_i(cc as usize), |o| o.get_i(cc as usize));
                    if T::IS_FLOAT {
                        let diff = ref_elev.to_f64() - target_height.to_f64();
                        pathdepth_f64 = pathdepth_f64.max(diff);
                    } else {
                        let diff = ref_elev.to_i128() - target_height.to_i128();
                        pathdepth_i128 = pathdepth_i128.max(diff);
                    }
                    cc = backlinks.get_i(cc as usize);
                    if eps_gradients {
                        target_height = target_height.next_down_float().unwrap();
                    }
                    pathlen += 1;
                }

                cc = pit_idx;
                target_height = dem.get_xy(c.x, c.y);

                let depth_ok = if T::IS_FLOAT {
                    pathdepth_f64 <= max_depth_f64
                } else {
                    pathdepth_i128 <= max_depth_i128
                };

                if pathlen <= max_path_len && depth_ok {
                    while cc != NO_BACK_LINK
                        && dem.get_i(cc as usize).rd_cmp(target_height) != Ordering::Less
                    {
                        dem.set_i(cc as usize, target_height);
                        cc = backlinks.get_i(cc as usize);
                        if eps_gradients {
                            target_height = target_height.next_down_float().unwrap();
                        }
                    }
                } else if mode == LindsayMode::ConstrainedBreaching {
                    let orig = orig_dem.as_ref().unwrap();
                    let mut steps = 0u32;
                    while cc != NO_BACK_LINK
                        && steps < max_path_len
                        && dem.get_i(cc as usize).rd_cmp(target_height) != Ordering::Less
                    {
                        let orig_e = orig.get_i(cc as usize);
                        let min_allowed = if T::IS_FLOAT {
                            let orig_f = orig_e.to_f64();
                            let limit_f = target_height.to_f64() + max_depth_f64;
                            if orig_f > limit_f {
                                T::from_f64(orig_f - max_depth_f64)
                            } else {
                                target_height
                            }
                        } else {
                            let orig_i = orig_e.to_i128();
                            let limit_i = target_height.to_i128() + max_depth_i128;
                            if orig_i > limit_i {
                                T::from_i128_clamped(orig_i - max_depth_i128)
                            } else {
                                target_height
                            }
                        };
                        let new_e = dem.get_i(cc as usize).rd_min(min_allowed);
                        dem.set_i(cc as usize, new_e);
                        if eps_gradients {
                            target_height = target_height.next_down_float().unwrap();
                        }
                        cc = backlinks.get_i(cc as usize);
                        steps += 1;
                    }
                    if steps == 0 && !fill_depressions {
                        dem.set_i(pit_idx as usize, orig.get_i(pit_idx as usize));
                    }
                } else if !fill_depressions {
                    // Selective breaching rejected this pit and filling is disabled:
                    // restore the pit's original elevation instead of leaving it filled.
                    let orig = orig_dem.as_ref().unwrap();
                    dem.set_i(pit_idx as usize, orig.get_i(pit_idx as usize));
                }
            }

            total_pits -= 1;
            if total_pits == 0 {
                break;
            }
        }

        let ci = dem.xy_to_i(c.x as usize, c.y as usize) as u32;
        for n in 1..=8 {
            let nx = c.x + D8X[n];
            let ny = c.y + D8Y[n];
            if !dem.in_grid(nx, ny)
                || dem.is_no_data_xy(nx, ny)
                || visited.get_xy(nx, ny) != UNVISITED
            {
                continue;
            }
            let my_e = dem.get_xy(nx, ny);
            pq.emplace(nx, ny, my_e);
            let ni = dem.xy_to_i(nx as usize, ny as usize) as u32;
            if fill_depressions {
                flood_array.push(ni);
            }
            visited.set_xy(nx, ny, VISITED);
            backlinks.set_xy(nx, ny, ci);
        }
    }

    if mode != LindsayMode::CompleteBreaching && fill_depressions {
        for f in flood_array {
            let parent = backlinks.get_i(f as usize);
            if parent == NO_BACK_LINK {
                continue;
            }
            let pe = dem.get_i(parent as usize);
            if dem.get_i(f as usize).rd_cmp(pe) != Ordering::Greater {
                if eps_gradients {
                    dem.set_i(f as usize, pe.next_up_float().unwrap());
                } else {
                    dem.set_i(f as usize, pe);
                }
            }
        }
    }

    Ok(())
}
