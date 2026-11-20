# Command-Line Interface

The `richdem` binary consolidates RichDEM's `rd_*` command-line utilities into a single pure-Rust executable with built-in GeoTIFF/COG/AAIGrid I/O via `terrio`.

```sh
cargo install richdem-cli
```

## Subcommands

### Hydrologic Conditioning & Depressions

| Subcommand | Description | Key Options |
|---|---|---|
| `depressions-flood <IN> <OUT>` | Priority-Flood depression filling | `-g/--gradient`, `-t/--topology <d8\|d4>`, `-c/--compress` |
| `depressions-breach <IN> <OUT>` | Lindsay (2016) depression breaching | `-m/--mode <complete\|selective\|constrained>`, `--max-length`, `--max-depth`, `--fill-depressions`, `-g/--gradient`, `-t/--topology`, `-c/--compress` |
| `depressions-has <IN>` | Prints `true` or `false` if DEM has depressions | `-t/--topology <d8\|d4>` |
| `depressions-mask <IN> <OUT>` | Outputs pit mask (`1`=pit, `0`=non-pit, `3`=NoData) | `-t/--topology <d8\|d4>`, `-c/--compress` |
| `resolve-flats <IN> <OUT>` | Barnes (2014) flat resolution via epsilon gradients | `-c/--compress` |

### Flow Directions, Accumulation & Routing

| Subcommand | Description | Key Options |
|---|---|---|
| `d8-flowdirs <IN> <OUT>` | D8 steepest-descent flow directions | `-c/--compress` |
| `flood-for-flowdirs <IN> <OUT>` | Implicit depression filling + D8 flow directions | `-c/--compress` |
| `flow-accumulation <IN> <OUT>` | Flow accumulation from DEM or D8 flowdirs | `-m/--method <D8\|D4\|Dinf\|Quinn\|Holmgren\|Freeman\|Rho8\|Rho4>`, `-e/--exponent`, `--from-d8`, `-c/--compress` |
| `depression-hierarchy <IN>` | Builds and prints a Depression Hierarchy | `-l/--labels-out`, `-f/--flowdirs-out`, `-t/--topology` |
| `fill-spill-merge <IN> <OUT>` | Fill-Spill-Merge hydrological water routing | `-w/--initial-water <F64>`, `--wtd-in <PATH>`, `-c/--compress` |
| `loop-check <IN>` | Checks a D8 flow direction raster for loops | — |
| `taudem-d8-to-richdem-d8 <IN> <OUT>` | Converts TauDEM D8 to RichDEM D8 | — |
| `arc-d8-to-richdem-d8 <IN> <OUT>` | Converts ArcGIS D8 to RichDEM D8 | — |

### Terrain Attributes & Raster Inspection

| Subcommand | Description | Key Options |
|---|---|---|
| `terrain-property <IN> <OUT>` | Slope, aspect, and curvature | `-a/--attrib <NAME>`, `-z/--zscale <F32>`, `-c/--compress` |
| `surface-area <IN>` | Topographic vs planar surface area (Jenness 2004) | `-z/--zscale <F64>` |
| `info <IN>` | Prints dimensions, dtype, NoData, geotransform, CRS, and `PROCESSING_HISTORY` | — |
| `compare <FIRST> <SECOND>` | Cell-by-cell and metadata diff (max, min, avg, RMS) | — |
| `geotransform <IN>` | Displays or sets the 6-element geotransform | `--set V0 V1 V2 V3 V4 V5`, `-o/--output`, `--in-place` |
| `no-data <IN>` | Displays or sets the NoData value | `--set <VAL>`, `-o/--output`, `--in-place` |
| `projection <IN>` | Displays or sets CRS (EPSG, WKT, PROJ, or `--from`) | `--set <CRS>`, `--from <PATH>`, `-o/--output`, `--in-place` |
| `processing-history <IN>` | Prints `PROCESSING_HISTORY` metadata | — |
| `hist <IN>` | Prints raster value histogram | — |
| `expand-dimensions <IN> <OUT> <W> <H>` | Pads raster to `(W, H)` with NoData | — |
| `raster-display <IN>` | Prints raster grid to terminal | — |
| `raster-inspect <IN> <X> <Y> <W> <H>` | Prints a `(W × H)` subregion starting at `(X, Y)` | — |

## Example Pipeline

```sh
richdem depressions-flood dem.tif filled.tif --gradient --compress
richdem flow-accumulation filled.tif accum.tif --method Holmgren --exponent 4.0 --compress
richdem terrain-property filled.tif slope.tif --attrib slope_degrees --compress
richdem compare dem.tif filled.tif
richdem processing-history accum.tif
```
