# Coming from RichDEM

`richdem-rs` is designed as a drop-in, pure-Rust replacement for Richard Barnes's C++ [RichDEM](https://github.com/r-barnes/richdem) and its Python wrapper (`richdem` / `richdem2`).

## Python API Compatibility

In most scripts, migrating from `richdem` or `richdem2` requires changing only the import statement:

```python
# Before
# import richdem as rd

# After
import richdem_rs as rd

dem = rd.LoadGDAL("dem.tif")
filled = rd.FillDepressions(dem, epsilon=True)
accum = rd.FlowAccumulation(filled, method="Dinf")
slope = rd.TerrainAttribute(filled, attrib="slope_degrees")
rd.SaveGDAL("accum.tif", accum, compress=True)
```

### What Stays the Same

- `rdarray` and `rd3array` NumPy subclasses carrying `no_data`, `geotransform`, `projection`, and `metadata` (including `PROCESSING_HISTORY`).
- Core functions: `LoadGDAL`, `SaveGDAL`, `FillDepressions`, `BreachDepressions`, `ResolveFlats`, `FlowAccumulation`, `FlowProportions`, `FlowAccumFromProps`, `TerrainAttribute`, `rdShow`.
- Flow direction and depression hierarchy helpers: `flow_accumulation_from_d8`, `convert_arc_flowdirs_to_richdem_d8`, `convert_taudem_d8_to_richdem_d8`, `get_new_depression_hierarchy_labels`, `get_depression_hierarchy`, `fill_spill_merge`, `generate_perlin_terrain`.
- Bit-for-bit agreement (`Max |diff| = 0.00e+00`) against C++ `richdem2` on Zhou (2016) filling, Barnes (2014) epsilon filling, Lindsay (2016) breaching, Barnes (2014) flat resolution, D8 and Quinn flow accumulation, slope, and curvature. (For `Dinf`, `f32` `atan2` ULP rounding yields ~10⁻⁶ relative difference at large outlet accumulations.)

### What Improves

- **No GDAL or C++ toolchain needed:** `LoadGDAL` and `SaveGDAL` are backed by pure-Rust [`terrio`](https://terrio.dev) — `pip install richdem-rs` works cleanly on Python 3.11–3.14 without conda or system `libgdal`.
- **Up to 3.5× faster** on key operations (`FillDepressions` 1.24–1.83×, `ResolveFlats` ~2.0×, `TerrainAttribute` 1.42–3.54×) with native `rayon` multithreading (note: single-threaded `BreachDepressions` at ~0.89–0.92× and D8 `FlowAccumulation` at ~0.86–0.91× are currently slightly slower than C++ `richdem2`; see [Benchmarks](benchmarks.md)).
- **No OpenMP runtime quirks:** Multithreading uses Rust's `rayon` thread pool without `libgomp` / `libomp` linking issues across macOS, Linux, and Windows.

## CLI Command Mapping (`rd_*` → `richdem`)

RichDEM's standalone `rd_*` executables map directly to subcommands of the single `richdem` binary:

| Upstream RichDEM tool | `richdem` CLI subcommand |
|---|---|
| `rd_depressions_flood` / `rd_depression_filling` | `richdem depressions-flood` |
| `rd_depressions_breach` | `richdem depressions-breach` |
| `rd_depressions_has` | `richdem depressions-has` |
| `rd_depressions_mask` | `richdem depressions-mask` |
| `rd_d8_flowdirs` | `richdem d8-flowdirs` |
| `rd_flood_for_flowdirs` | `richdem flood-for-flowdirs` |
| `rd_flow_accumulation` | `richdem flow-accumulation` |
| `rd_terrain_property` | `richdem terrain-property` |
| `rd_surface_area` | `richdem surface-area` |
| `rd_depression_hierarchy` | `richdem depression-hierarchy` |
| `rd_fill_spill_merge` | `richdem fill-spill-merge` |
| `rd_info` | `richdem info` |
| `rd_compare` | `richdem compare` |
| `rd_geotransform` | `richdem geotransform` |
| `rd_no_data` | `richdem no-data` |
| `rd_projection` | `richdem projection` |
| `rd_processing_history` | `richdem processing-history` |
| `rd_hist` | `richdem hist` |
| `rd_loop_check` | `richdem loop-check` |
| `rd_expand_dimensions` | `richdem expand-dimensions` |
| `rd_raster_display` | `richdem raster-display` |
| `rd_raster_inspect` | `richdem raster-inspect` |
| `rd_taudem_d8_to_richdem_d8` | `richdem taudem-d8-to-richdem-d8` |

## What Is Not Ported Yet

- **Distributed MPI tiling (`Barnes 2016/2017` trillion-cell cluster mode):** `richdem-rs` provides single-node multi-core scaling via `rayon`, not multi-node MPI cluster partitioning.
