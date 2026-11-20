//! Reproducible pure-Rust release benchmarks for `richdem`.
//!
//! Run with:
//! ```sh
//! cargo run --release --example benchmark
//! ```

use std::time::{Duration, Instant};

use richdem::{
    Array2D, FlowAccumMethod, TerrainAttr, Topology, complete_breaching_lindsay2016,
    fill_spill_merge, flow_accumulation_by_method, generate_perlin_terrain,
    get_depression_hierarchy, get_new_depression_hierarchy_labels, load_raster,
    priority_flood_barnes2014, priority_flood_epsilon_barnes2014, priority_flood_wei2018,
    priority_flood_zhou2016, resolve_flats_epsilon, save_raster, terrain_attribute,
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

fn print_row(name: &str, cells: usize, dur: Duration) {
    let ms = dur.as_secs_f64() * 1e3;
    let mcells_s = (cells as f64) / dur.as_secs_f64() / 1e6;
    println!("| {name:<36} | {ms:>9.1} ms | {mcells_s:>9.2} M cells/s |");
}

fn run_suite(size: usize, iters: usize) {
    let cells = size * size;
    println!(
        "\n=== Pure Rust Benchmark: {size} x {size} ({cells} cells, f64, median of {iters} runs) ==="
    );
    println!(
        "| {:<36} | {:>12} | {:>19} |",
        "Algorithm", "Median Time", "Throughput"
    );
    println!("|{:-<38}|{:-<14}|{:-<21}|", "", "", "");

    let dem = generate_perlin_terrain(size, 42);

    let d_zhou = bench_median(1, iters, || {
        let mut work = dem.clone();
        priority_flood_zhou2016(&mut work);
    });
    print_row("PriorityFlood (Zhou2016)", cells, d_zhou);

    let d_wei = bench_median(1, iters, || {
        let mut work = dem.clone();
        priority_flood_wei2018(&mut work);
    });
    print_row("PriorityFlood (Wei2018)", cells, d_wei);

    let d_barnes = bench_median(1, iters, || {
        let mut work = dem.clone();
        priority_flood_barnes2014(&mut work, Topology::D8);
    });
    print_row("PriorityFlood (Barnes2014)", cells, d_barnes);

    let d_eps = bench_median(1, iters, || {
        let mut work = dem.clone();
        priority_flood_epsilon_barnes2014(&mut work, Topology::D8).unwrap();
    });
    print_row("PriorityFlood+Epsilon (Barnes2014)", cells, d_eps);

    let d_breach = bench_median(1, iters, || {
        let mut work = dem.clone();
        complete_breaching_lindsay2016(&mut work, Topology::D8);
    });
    print_row("BreachDepressions (Lindsay2016)", cells, d_breach);

    // Pre-filled DEM (with flats) for ResolveFlats, and epsilon-filled DEM for flow accumulation
    let mut filled_flat = dem.clone();
    priority_flood_zhou2016(&mut filled_flat);

    let d_flats = bench_median(1, iters, || {
        let mut work = filled_flat.clone();
        resolve_flats_epsilon(&mut work).unwrap();
    });
    print_row("ResolveFlats (Barnes2014 Epsilon)", cells, d_flats);

    let mut filled_eps = dem.clone();
    priority_flood_epsilon_barnes2014(&mut filled_eps, Topology::D8).unwrap();

    let d_d8 = bench_median(1, iters, || {
        let mut accum = Array2D::make_from_template(&filled_eps, 1.0f64);
        flow_accumulation_by_method(&filled_eps, &mut accum, FlowAccumMethod::D8, None).unwrap();
    });
    print_row("FlowAccumulation (D8)", cells, d_d8);

    let d_dinf = bench_median(1, iters, || {
        let mut accum = Array2D::make_from_template(&filled_eps, 1.0f64);
        flow_accumulation_by_method(&filled_eps, &mut accum, FlowAccumMethod::Dinf, None).unwrap();
    });
    print_row("FlowAccumulation (Dinf / Tarboton)", cells, d_dinf);

    let d_quinn = bench_median(1, iters, || {
        let mut accum = Array2D::make_from_template(&filled_eps, 1.0f64);
        flow_accumulation_by_method(&filled_eps, &mut accum, FlowAccumMethod::Quinn, None).unwrap();
    });
    print_row("FlowAccumulation (Quinn MFD)", cells, d_quinn);

    let d_slope = bench_median(1, iters, || {
        let _s = terrain_attribute(&filled_eps, TerrainAttr::SlopeDegrees, 1.0);
    });
    print_row("TerrainAttribute (slope_degrees)", cells, d_slope);

    let d_curv = bench_median(1, iters, || {
        let _c = terrain_attribute(&filled_eps, TerrainAttr::Curvature, 1.0);
    });
    print_row("TerrainAttribute (curvature)", cells, d_curv);

    let d_dh_fsm = bench_median(1, iters, || {
        let mut labels = get_new_depression_hierarchy_labels(dem.width(), dem.height(), None);
        let mut flowdirs = Array2D::new(dem.width(), dem.height(), 0i8);
        let mut deps =
            get_depression_hierarchy(&dem, &mut labels, &mut flowdirs, Topology::D8).unwrap();
        let mut wtd = Array2D::make_from_template(&dem, 1.0f64);
        fill_spill_merge(&dem, &labels, &flowdirs, &mut deps, &mut wtd).unwrap();
    });
    print_row("DepressionHierarchy + FillSpillMerge", cells, d_dh_fsm);

    // Pure-Rust GeoTIFF I/O via terrio
    let tmp_dir = tempfile::tempdir().unwrap();
    let tmp_path = tmp_dir.path().join(format!("richdem_rs_bench_{size}.tif"));
    let d_write = bench_median(1, iters, || {
        save_raster(&filled_eps, &tmp_path, None, false).unwrap();
    });
    print_row("GeoTIFF Write (terrio, uncompressed)", cells, d_write);

    let d_read = bench_median(1, iters, || {
        let _r = load_raster::<f64>(&tmp_path, None).unwrap();
    });
    print_row("GeoTIFF Read (terrio, uncompressed)", cells, d_read);
    drop(tmp_dir);
}

fn main() {
    println!("richdem-rs v{} benchmark suite", richdem::VERSION);
    run_suite(1000, 3);
    run_suite(2000, 3);
}
