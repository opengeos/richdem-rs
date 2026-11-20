//! Priority-Flood depression filling (Wei, Zhou, and Fu, 2018 variant).
//!
//! Reference: Wei, H., Zhou, G., Fu, S., 2018. Efficient Priority-Flood depression
//! filling in raster digital elevation models. International Journal of Digital Earth,
//! 12(4), 415-427. doi:10.1080/17538947.2018.1429503

use std::cmp::Ordering;
use std::collections::VecDeque;

use crate::array2d::Array2D;
use crate::constants::{D8X, D8Y};
use crate::grid_cell::{GridCellZ, GridCellZPq};
use crate::num::DemNum;

fn init_priority_que<T: DemNum>(
    dem: &Array2D<T>,
    flag: &mut Array2D<u8>,
    priority_queue: &mut GridCellZPq<T>,
) {
    let w = dem.width();
    let h = dem.height();
    for y in 0..h {
        for x in 0..w {
            if flag.get(x, y) != 0 {
                continue;
            }
            if dem.is_no_data(x, y) {
                flag.set(x, y, 1);
                for n in 1..=8 {
                    let nx = x as i32 + D8X[n];
                    let ny = y as i32 + D8Y[n];
                    if !dem.in_grid(nx, ny) || flag.get_xy(nx, ny) != 0 {
                        continue;
                    }
                    if !dem.is_no_data_xy(nx, ny) {
                        priority_queue.push(GridCellZ::new(nx, ny, dem.get_xy(nx, ny)));
                        flag.set_xy(nx, ny, 1);
                    }
                }
            } else if dem.is_edge_cell(x, y) {
                priority_queue.push(GridCellZ::new(x as i32, y as i32, dem.get(x, y)));
                flag.set(x, y, 1);
            }
        }
    }
}

fn process_trace_que<T: DemNum>(
    dem: &Array2D<T>,
    flag: &mut Array2D<u8>,
    trace_queue: &mut VecDeque<GridCellZ<T>>,
    priority_queue: &mut GridCellZPq<T>,
) {
    let mut potential_queue = VecDeque::<GridCellZ<T>>::new();
    const INDEX_THRESHOLD: usize = 2;

    while let Some(node) = trace_queue.pop_front() {
        let mut mask = [[false; 5]; 5];
        for n in 1..=8 {
            let nx = node.x + D8X[n];
            let ny = node.y + D8Y[n];
            if !dem.in_grid(nx, ny) || flag.get_xy(nx, ny) != 0 {
                continue;
            }
            let nz = dem.get_xy(nx, ny);
            if nz.rd_cmp(node.z) == Ordering::Greater {
                trace_queue.push_back(GridCellZ::new(nx, ny, nz));
                flag.set_xy(nx, ny, 1);
            } else {
                let mut have_spill_path = false;
                for k in 1..=8 {
                    let nnx = nx + D8X[k];
                    let nny = ny + D8Y[k];
                    let my = (nny - node.y + 2) as usize;
                    let mx = (nnx - node.x + 2) as usize;
                    let mask_hit = my < 5 && mx < 5 && mask[my][mx];
                    let flagged_lower = dem.in_grid(nnx, nny)
                        && flag.get_xy(nnx, nny) != 0
                        && dem.get_xy(nnx, nny).rd_cmp(node.z) == Ordering::Less;
                    if mask_hit || flagged_lower {
                        mask[(ny - node.y + 2) as usize][(nx - node.x + 2) as usize] = true;
                        have_spill_path = true;
                        break;
                    }
                }
                if !have_spill_path {
                    if n < INDEX_THRESHOLD {
                        potential_queue.push_back(node);
                    } else {
                        priority_queue.push(node);
                    }
                    break;
                }
            }
        }
    }

    while let Some(node) = potential_queue.pop_front() {
        for n in 1..=8 {
            let nx = node.x + D8X[n];
            let ny = node.y + D8Y[n];
            if !dem.in_grid(nx, ny) || flag.get_xy(nx, ny) != 0 {
                continue;
            }
            priority_queue.push(node);
            break;
        }
    }
}

fn process_pit<T: DemNum>(
    dem: &mut Array2D<T>,
    flag: &mut Array2D<u8>,
    depression_que: &mut VecDeque<GridCellZ<T>>,
    trace_queue: &mut VecDeque<GridCellZ<T>>,
) {
    while let Some(node) = depression_que.pop_front() {
        for n in 1..=8 {
            let nx = node.x + D8X[n];
            let ny = node.y + D8Y[n];
            if !dem.in_grid(nx, ny) || flag.get_xy(nx, ny) != 0 {
                continue;
            }
            let i_spill = dem.get_xy(nx, ny);
            if i_spill.rd_cmp(node.z) == Ordering::Greater {
                flag.set_xy(nx, ny, 1);
                trace_queue.push_back(GridCellZ::new(nx, ny, i_spill));
                continue;
            }
            flag.set_xy(nx, ny, 1);
            dem.set_xy(nx, ny, node.z);
            depression_que.push_back(GridCellZ::new(nx, ny, node.z));
        }
    }
}

/// Fills all D8 depressions in `dem` using Wei, Zhou, and Fu (2018) Priority-Flood.
pub fn priority_flood_wei2018<T: DemNum>(dem: &mut Array2D<T>) {
    let w = dem.width();
    let h = dem.height();
    if w == 0 || h == 0 {
        return;
    }

    let mut trace_queue = VecDeque::<GridCellZ<T>>::new();
    let mut depression_que = VecDeque::<GridCellZ<T>>::new();
    let mut flag = Array2D::<u8>::new(w, h, 0);
    let mut priority_queue = GridCellZPq::new();

    init_priority_que(dem, &mut flag, &mut priority_queue);

    while let Some(tmp_node) = priority_queue.pop() {
        for n in 1..=8 {
            let nx = tmp_node.x + D8X[n];
            let ny = tmp_node.y + D8Y[n];
            if !dem.in_grid(nx, ny) || flag.get_xy(nx, ny) != 0 {
                continue;
            }
            let i_spill = dem.get_xy(nx, ny);
            if i_spill.rd_cmp(tmp_node.z) != Ordering::Greater {
                dem.set_xy(nx, ny, tmp_node.z);
                flag.set_xy(nx, ny, 1);
                depression_que.push_back(GridCellZ::new(nx, ny, tmp_node.z));
                process_pit(dem, &mut flag, &mut depression_que, &mut trace_queue);
            } else {
                flag.set_xy(nx, ny, 1);
                trace_queue.push_back(GridCellZ::new(nx, ny, i_spill));
            }
            process_trace_que(dem, &mut flag, &mut trace_queue, &mut priority_queue);
        }
    }
}
