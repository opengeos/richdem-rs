# Benchmarks

Benchmarks were measured on synthetic Perlin-noise `float64` DEMs (`seed = 42`, warmup + median of 3 runs) on an **Intel Xeon w7-3445** running Linux (`x86_64`), compiled with **`rustc 1.100.0`** (`--release`), comparing **`richdem-rs 0.1.0`** against **`richdem2 2.4.3`** (C++ / pybind11).

## Python Head-to-Head (`richdem-rs` vs C++ `richdem2`)

Measured via `scripts/benchmark.py` on identical `float64` NumPy `rdarray` inputs, also checking the maximum absolute difference (`Max |diff|`) between outputs:

### 1000 × 1000 (1,000,000 cells)

| Operation | `richdem2` (C++) | `richdem-rs` (Rust) | Speedup | Max \|diff\| |
|---|---:|---:|---:|---:|
| `FillDepressions` (Zhou2016) | 41.6 ms | **26.8 ms** | **1.56×** | `0.00e+00` |
| `FillDepressions` (`epsilon=True`) | 87.0 ms | **66.5 ms** | **1.31×** | `0.00e+00` |
| `BreachDepressions` (Lindsay2016) | 89.7 ms | 97.9 ms | 0.92× | `0.00e+00` |
| `ResolveFlats` (Barnes2014) | 354.7 ms | **175.5 ms** | **2.02×** | `0.00e+00` |
| `FlowAccumulation` (`D8`) | 43.4 ms | 50.5 ms | 0.86× | `0.00e+00` |
| `FlowAccumulation` (`Dinf`) | 123.3 ms | **120.5 ms** | **1.02×** | `1.81e-02` |
| `FlowAccumulation` (`Quinn`) | 105.8 ms | **102.4 ms** | **1.03×** | `0.00e+00` |
| `TerrainAttribute` (`slope_degrees`) | 12.7 ms | **3.6 ms** | **3.54×** | `0.00e+00` |
| `TerrainAttribute` (`curvature`) | 6.2 ms | **3.7 ms** | **1.69×** | `0.00e+00` |

### 2000 × 2000 (4,000,000 cells)

| Operation | `richdem2` (C++) | `richdem-rs` (Rust) | Speedup | Max \|diff\| |
|---|---:|---:|---:|---:|
| `FillDepressions` (Zhou2016) | 170.3 ms | **93.0 ms** | **1.83×** | `0.00e+00` |
| `FillDepressions` (`epsilon=True`) | 400.2 ms | **323.2 ms** | **1.24×** | `0.00e+00` |
| `BreachDepressions` (Lindsay2016) | 397.6 ms | 447.0 ms | 0.89× | `0.00e+00` |
| `ResolveFlats` (Barnes2014) | 2,487.3 ms | **1,221.3 ms** | **2.04×** | `0.00e+00` |
| `FlowAccumulation` (`D8`) | 215.3 ms | 235.4 ms | 0.91× | `0.00e+00` |
| `FlowAccumulation` (`Dinf`) | 511.6 ms | 517.8 ms | 0.99× | `1.27e-01` |
| `FlowAccumulation` (`Quinn`) | 467.2 ms | 472.0 ms | 0.99× | `0.00e+00` |
| `TerrainAttribute` (`slope_degrees`) | 44.7 ms | **14.0 ms** | **3.19×** | `0.00e+00` |
| `TerrainAttribute` (`curvature`) | 20.6 ms | **14.5 ms** | **1.42×** | `0.00e+00` |

!!! note "Note on `Dinf` `Max |diff|`"
    `Dinf` flow proportions are stored in `f32`, so tiny `atan2` ULP rounding differences compound slightly along long upstream drainage paths (relative error ~10⁻⁶ on outlet accumulations of ~10⁵–10⁶ cells). All other methods match C++ `richdem2` bit-for-bit (`0.00e+00`).

## Pure Rust Native Throughput

Measured directly in Rust without Python/NumPy copy overhead via `crates/richdem/examples/benchmark.rs`:

| Algorithm | 1000 × 1000 (1M cells) | 2000 × 2000 (4M cells) | Throughput (4M) |
|---|---:|---:|---:|
| `PriorityFlood` (Wei2018) | 17.8 ms | 76.7 ms | **52.2 M cells/s** |
| `PriorityFlood` (Zhou2016) | 23.2 ms | 92.3 ms | **43.3 M cells/s** |
| `PriorityFlood` (Barnes2014) | 73.2 ms | 286.8 ms | 14.0 M cells/s |
| `PriorityFlood+Epsilon` (Barnes2014) | 63.8 ms | 308.8 ms | 13.0 M cells/s |
| `BreachDepressions` (Lindsay2016) | 95.2 ms | 442.8 ms | 9.0 M cells/s |
| `ResolveFlats` (Barnes2014 Epsilon) | 180.0 ms | 1,235.6 ms | 3.2 M cells/s |
| `FlowAccumulation` (`D8`) | 44.1 ms | 229.7 ms | 17.4 M cells/s |
| `FlowAccumulation` (`Dinf` / Tarboton) | 117.0 ms | 512.3 ms | 7.8 M cells/s |
| `FlowAccumulation` (`Quinn` MFD) | 102.0 ms | 470.2 ms | 8.5 M cells/s |
| `TerrainAttribute` (`slope_degrees`) | 2.1 ms | 6.3 ms | **637.6 M cells/s** |
| `TerrainAttribute` (`curvature`) | 1.7 ms | 5.3 ms | **758.3 M cells/s** |
| `DepressionHierarchy` + `FillSpillMerge` | 215.6 ms | 1,009.3 ms | 4.0 M cells/s |
| GeoTIFF Write (`terrio`, uncompressed) | 5.3 ms | 21.8 ms | **183.3 M cells/s** |
| GeoTIFF Read (`terrio`, uncompressed) | 5.4 ms | 25.8 ms | **155.1 M cells/s** |

## Reproducing Locally

```sh
# Pure Rust release benchmark
cargo run --release -p richdem --example benchmark

# Python head-to-head comparison against C++ richdem2
cd python
python -m venv .venv && source .venv/bin/activate
pip install maturin numpy richdem2
maturin develop --release
python ../scripts/benchmark.py
```

For multi-core `rayon` thread scaling across 1T–40T, see [Multithreading](parallelism.md).
