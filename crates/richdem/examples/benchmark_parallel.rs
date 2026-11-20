//! Reproducible single-threaded vs. Rayon multithreaded scaling benchmark for `richdem`.
//!
//! Run with:
//! ```sh
//! cargo run --release -p richdem --example benchmark_parallel
//! ```
//!
//! Also compiles without `parallel` to benchmark the purely sequential build:
//! ```sh
//! cargo run --release -p richdem --no-default-features --example benchmark_parallel
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use richdem::{
    PerimType, TerrainAttr, Topology, d8_flow_directions, dem_surface_area, dinf_flow_directions,
    find_flats, fm_d8, fm_freeman, fm_holmgren, fm_quinn, fm_tarboton, generate_perlin_terrain,
    perimeter, priority_flood_epsilon_barnes2014, priority_flood_zhou2016, terrain_attribute,
};

fn bench_median<F: FnMut()>(warmup: usize, iters: usize, mut f: F) -> Duration {
    for _ in 0..warmup {
        f();
    }
    let mut times = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        f();
        times.push(start.elapsed());
    }
    times.sort();
    times[times.len() / 2]
}

#[cfg(feature = "parallel")]
fn run_with_threads<F: FnMut() + Send>(
    threads: usize,
    warmup: usize,
    iters: usize,
    f: F,
) -> Duration {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("failed to build rayon thread pool");
    pool.install(|| bench_median(warmup, iters, f))
}

fn run_suite(size: usize, iters: usize) {
    let cells = size * size;
    let dem = generate_perlin_terrain(size, 42);
    let mut filled_flat = dem.clone();
    priority_flood_zhou2016(&mut filled_flat);
    let mut filled_eps = dem.clone();
    priority_flood_epsilon_barnes2014(&mut filled_eps, Topology::D8).unwrap();

    #[cfg(feature = "parallel")]
    {
        let max_threads = rayon::current_num_threads();
        let mut counts = vec![1usize];
        for t in [4usize, 8, 16] {
            if t < max_threads {
                counts.push(t);
            }
        }
        if max_threads > 1 {
            counts.push(max_threads);
        }

        println!(
            "\n=== Rayon Multithreading Benchmark: {size} x {size} ({cells} cells, f64, median of {iters} runs) ==="
        );
        print!("| {:<32} |", "Kernel");
        for &t in &counts {
            print!(" {:>9} |", format!("{t}T"));
        }
        println!(" {:>9} |", "Speedup");

        print!("|{:-<34}|", "");
        for _ in &counts {
            print!("{:-<11}|", "");
        }
        println!("{:-<11}|", "");

        let report = |name: &str, f: &(dyn Fn() + Sync)| {
            let mut durs = Vec::with_capacity(counts.len());
            for &t in &counts {
                durs.push(run_with_threads(t, 1, iters, f));
            }
            let base = durs[0].as_secs_f64();
            let best = durs.last().unwrap().as_secs_f64();
            let speedup = base / best;
            print!("| {name:<32} |");
            for d in &durs {
                print!(" {:>6.1} ms |", d.as_secs_f64() * 1e3);
            }
            println!(" {:>8.2}x |", speedup);
        };

        report("d8_flow_directions", &|| {
            black_box(d8_flow_directions(&filled_eps));
        });
        report("dinf_flow_directions", &|| {
            black_box(dinf_flow_directions(&filled_eps));
        });
        report("fm_d8 (O'Callaghan 1984)", &|| {
            black_box(fm_d8(&filled_eps));
        });
        report("fm_tarboton (Dinf)", &|| {
            black_box(fm_tarboton(&filled_eps));
        });
        report("fm_quinn (Quinn 1991)", &|| {
            black_box(fm_quinn(&filled_eps));
        });
        report("fm_holmgren (x=4.0)", &|| {
            black_box(fm_holmgren(&filled_eps, 4.0));
        });
        report("fm_freeman (x=1.1)", &|| {
            black_box(fm_freeman(&filled_eps, 1.1));
        });
        report("find_flats", &|| {
            black_box(find_flats(&filled_flat));
        });
        report("ta_slope_degrees (Horn 1981)", &|| {
            black_box(terrain_attribute(
                &filled_eps,
                TerrainAttr::SlopeDegrees,
                1.0,
            ));
        });
        report("ta_curvature", &|| {
            black_box(terrain_attribute(&filled_eps, TerrainAttr::Curvature, 1.0));
        });
        report("dem_surface_area (Jenness 2004)", &|| {
            black_box(dem_surface_area(&filled_eps, 1.0).unwrap());
        });
        report("perimeter (SquareEdge)", &|| {
            black_box(perimeter(&filled_eps, PerimType::SquareEdge));
        });
    }

    #[cfg(not(feature = "parallel"))]
    {
        println!(
            "\n=== Serial Benchmark (feature `parallel` disabled): {size} x {size} ({cells} cells, f64, median of {iters} runs) ==="
        );
        println!("| {:<32} | {:>12} |", "Kernel", "Median Time");
        println!("|{:-<34}|{:-<14}|", "", "");
        let report = |name: &str, f: &dyn Fn()| {
            let d = bench_median(1, iters, f);
            println!("| {name:<32} | {:>9.1} ms |", d.as_secs_f64() * 1e3);
        };
        report("d8_flow_directions", &|| {
            black_box(d8_flow_directions(&filled_eps));
        });
        report("dinf_flow_directions", &|| {
            black_box(dinf_flow_directions(&filled_eps));
        });
        report("fm_d8 (O'Callaghan 1984)", &|| {
            black_box(fm_d8(&filled_eps));
        });
        report("fm_tarboton (Dinf)", &|| {
            black_box(fm_tarboton(&filled_eps));
        });
        report("fm_quinn (Quinn 1991)", &|| {
            black_box(fm_quinn(&filled_eps));
        });
        report("fm_holmgren (x=4.0)", &|| {
            black_box(fm_holmgren(&filled_eps, 4.0));
        });
        report("fm_freeman (x=1.1)", &|| {
            black_box(fm_freeman(&filled_eps, 1.1));
        });
        report("find_flats", &|| {
            black_box(find_flats(&filled_flat));
        });
        report("ta_slope_degrees (Horn 1981)", &|| {
            black_box(terrain_attribute(
                &filled_eps,
                TerrainAttr::SlopeDegrees,
                1.0,
            ));
        });
        report("ta_curvature", &|| {
            black_box(terrain_attribute(&filled_eps, TerrainAttr::Curvature, 1.0));
        });
        report("dem_surface_area (Jenness 2004)", &|| {
            black_box(dem_surface_area(&filled_eps, 1.0).unwrap());
        });
        report("perimeter (SquareEdge)", &|| {
            black_box(perimeter(&filled_eps, PerimType::SquareEdge));
        });
    }
}

fn main() {
    println!("richdem-rs v{} Rayon scaling benchmark", richdem::VERSION);
    run_suite(1000, 3);
    run_suite(2000, 3);
}
