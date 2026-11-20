# Multithreading

`richdem-rs` uses pure-Rust [`rayon`](https://docs.rs/rayon) multithreading under the default `parallel` Cargo feature — providing native multi-core scaling with zero OpenMP or C runtime dependencies.

## Controlling Thread Count

By default, `rayon` uses all available logical CPU cores. You can limit the thread count in Rust, Python, or the CLI using the standard `RAYON_NUM_THREADS` environment variable:

```sh
# Run the CLI or a Python script with 8 threads
RAYON_NUM_THREADS=8 richdem terrain-property dem.tif slope.tif --attrib slope_degrees
RAYON_NUM_THREADS=8 python my_analysis.py
```

To compile a strictly single-threaded Rust binary without `rayon`:

```toml
[dependencies]
richdem = { version = "0.1", default-features = false, features = ["terrio"] }
```

## Determinism

All parallelized kernels operate on independent output rows or collect per-row partial results in deterministic row order, ensuring identical results across thread counts and between `parallel` and single-threaded builds.

## What Runs in Parallel vs. Sequentially

- **Parallelized (multi-core via `rayon`):**
  - Terrain attributes (`terrain_attribute`: slope, aspect, curvature)
  - Flow directions and flow metrics (`d8_flow_directions`, `dinf_flow_directions`, `fm_d8`/`fm_d4`, `fm_tarboton`, `fm_quinn`, `fm_holmgren`, `fm_freeman`)
  - Flat detection and flat resolution passes (`find_flats`, flat edge discovery, flat flow/DEM alteration passes)
  - Surface metrics and hierarchy seed discovery (`dem_surface_area`, `perimeter`, `get_depression_hierarchy` ocean/land seed scans)
- **Inherently sequential:**
  - Priority-queue passes (Priority-Flood, Lindsay 2016 breaching, Depression Hierarchy flood merge, Fill-Spill-Merge, and topological flow accumulation propagation), which are already fast in single-threaded Rust (up to 52 M cells/s for Wei 2018 Priority-Flood).

## Multi-Core Scaling (Intel Xeon w7-3445, 40 Logical Cores)

Measured on synthetic Perlin-noise `f64` DEMs (median of 3 release runs, `1T` to `40T`) using `crates/richdem/examples/benchmark_parallel.rs`:

```sh
cargo run --release -p richdem --example benchmark_parallel
```

### 2000 × 2000 (4,000,000 cells)

| Kernel | 1T | 4T | 8T | 16T | 40T | Speedup (1T → 40T) |
|---|---:|---:|---:|---:|---:|---:|
| `d8_flow_directions` | 103.1 ms | 29.7 ms | 15.1 ms | 7.9 ms | 6.5 ms | **15.80×** |
| `dinf_flow_directions` | 384.7 ms | 107.0 ms | 59.0 ms | 30.1 ms | 23.8 ms | **16.18×** |
| `fm_d8` (O'Callaghan 1984) | 94.6 ms | 37.7 ms | 29.3 ms | 23.1 ms | 22.1 ms | **4.28×** |
| `fm_tarboton` (Dinf) | 409.9 ms | 119.7 ms | 70.8 ms | 46.4 ms | 39.9 ms | **10.28×** |
| `fm_quinn` (Quinn 1991) | 279.0 ms | 86.4 ms | 56.7 ms | 38.8 ms | 31.5 ms | **8.86×** |
| `fm_holmgren` (x=4.0) | 277.0 ms | 93.1 ms | 57.4 ms | 40.2 ms | 32.8 ms | **8.44×** |
| `fm_freeman` (x=1.1) | 277.6 ms | 86.7 ms | 55.0 ms | 38.5 ms | 30.7 ms | **9.05×** |
| `find_flats` | 27.4 ms | 6.5 ms | 3.7 ms | 2.2 ms | 1.9 ms | **14.40×** |
| `ta_slope_degrees` (Horn 1981) | 152.5 ms | 46.0 ms | 23.1 ms | 12.6 ms | 6.8 ms | **22.29×** |
| `ta_curvature` | 92.5 ms | 25.8 ms | 14.6 ms | 8.0 ms | 5.3 ms | **17.32×** |
| `dem_surface_area` (Jenness 2004) | 414.7 ms | 112.1 ms | 56.6 ms | 31.2 ms | 21.8 ms | **18.98×** |
| `perimeter` (SquareEdge) | 27.8 ms | 7.3 ms | 4.3 ms | 2.2 ms | 1.8 ms | **15.47×** |

!!! note "Distributed MPI"
    Upstream RichDEM's multi-node MPI cluster tiling (Barnes 2016/2017) is not currently implemented in `richdem-rs`.
