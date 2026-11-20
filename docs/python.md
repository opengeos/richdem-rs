# Python API Reference

The `richdem-rs` Python package (import name: `richdem_rs`) provides a drop-in NumPy API built with PyO3 and maturin, with pure-Rust raster I/O via `terrio`.

```sh
pip install richdem-rs
```

## Core Array Types

### `rdarray(array, meta_obj=None, no_data=None, dtype=None, order=None, geotransform=None)`

2D `numpy.ndarray` subclass carrying RichDEM geospatial metadata:

- `no_data`: sentinel value marking missing cells (required unless copied from `meta_obj`)
- `geotransform`: 6-element GDAL-style affine transform `[origin_x, pixel_w, rot_x, origin_y, rot_y, pixel_h]`
- `projection`: CRS string (EPSG, WKT, or PROJ)
- `metadata`: `dict[str, str]`, including `PROCESSING_HISTORY`

### `rd3array(array, meta_obj=None, no_data=None, order=None)`

3D `(height, width, 9)` `float32` `numpy.ndarray` subclass storing per-cell flow proportions returned by `FlowProportions`.

## Raster I/O

| Function | Signature | Description |
|---|---|---|
| `LoadGDAL` | `(filename, no_data=None) -> rdarray` | Reads GeoTIFF, COG, AAIGrid, ENVI, HFA, VRT via pure-Rust `terrio` (no GDAL needed) |
| `SaveGDAL` | `(filename, rda, compress=False) -> None` | Writes `rdarray` preserving dtype, `no_data`, `geotransform`, `projection`, and `metadata` |

## Hydrologic & Terrain Functions

| Function | Signature | Description |
|---|---|---|
| `FillDepressions` | `(dem, epsilon=False, in_place=False, topology="D8")` | Priority-Flood depression filling (optional epsilon downhill gradient) |
| `BreachDepressions` | `(dem, in_place=False, topology="D8")` | Lindsay (2016) complete depression breaching |
| `ResolveFlats` | `(dem, in_place=False)` | Barnes (2014) flat resolution via minimal floating-point increments |
| `FlowAccumulation` | `(dem, method, exponent=None, weights=None, in_place=False)` | Flow accumulation (`D8`, `D4`, `Dinf`, `Tarboton`, `Quinn`, `Holmgren`, `Freeman`, `Rho8`, `Rho4`) |
| `FlowProportions` | `(dem, method, exponent=None) -> rd3array` | Computes reusable `(H, W, 9)` flow proportions |
| `FlowAccumFromProps` | `(props, weights=None, in_place=False) -> rdarray` | Accumulates flow from precomputed `rd3array` proportions |
| `TerrainAttribute` | `(dem, attrib, zscale=1.0) -> rdarray` | Slope (`slope_riserun`, `slope_percentage`, `slope_degrees`, `slope_radians`), `aspect`, `curvature`, `planform_curvature`, `profile_curvature` |
| `flow_accumulation_from_d8` | `(d8) -> rdarray` | Accumulates interior cells directly from RichDEM D8 flow directions |
| `convert_arc_flowdirs_to_richdem_d8` | `(arc_flowdirs) -> rdarray` | Converts ArcGIS D8 codes (`1..128`) to RichDEM D8 codes (`1..8`) |
| `convert_taudem_d8_to_richdem_d8` | `(taudem_flowdirs) -> rdarray` | Converts TauDEM D8 codes to RichDEM D8 codes |
| `generate_perlin_terrain` | `(size, seed) -> rdarray` | Generates a deterministic square synthetic Perlin-noise DEM |
| `rdShow` | `(rda, ignore_colours=(), show=True, axes=True, cmap="gray", vmin=None, vmax=None, figsize=(4, 4))` | Quick matplotlib visualization with 2–98% percentile stretch |

## Depression Hierarchy & Fill-Spill-Merge

| Function / Constant | Description |
|---|---|
| `get_new_depression_hierarchy_labels(shape, no_data=NO_DEP, geotransform=None)` | Creates a `uint32` labels array initialized with `OCEAN` borders and `NO_DEP` interior |
| `get_depression_hierarchy(dem, labels, topology="D8")` | Builds `(deps, flowdirs)` and updates `labels` in place |
| `fill_spill_merge(dem, labels, flowdirs, deps, wtd)` | Routes surface water through `deps`, updating `deps` and `wtd` in place |
| `depression_hierarchy.Depression` | Depression node class exposing `pit_cell`, `out_cell`, `parent`, `pit_elev`, `out_elev`, `cell_count`, `dep_vol`, `water_vol`, etc. |
| `depression_hierarchy.{OCEAN, NO_DEP, NO_PARENT, NO_VALUE}` | Sentinel constants |
