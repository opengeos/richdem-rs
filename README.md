# richdem-rs

[![CI](https://github.com/opengeos/richdem-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/opengeos/richdem-rs/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

High-performance Digital Elevation Model (DEM) hydrologic and terrain analysis in **pure Rust**, with GDAL-free geospatial raster I/O powered by [`terrio`](https://github.com/opengeos/terrio).

> **Credits & Attribution:**
> `richdem-rs` is a pure-Rust reimplementation of [**RichDEM**](https://github.com/r-barnes/richdem), originally designed, authored, and published by [**Richard Barnes**](https://github.com/r-barnes) (`rbarnes@umn.edu`). All credit for the library design, hydrologic algorithms, data structures, and test suite belongs to Richard Barnes and the original algorithm authors listed under [Publications & Credits](#publications--credits).

## Highlights

- **Full algorithmic parity with RichDEM:**
  - **Depression filling:** Priority-Flood (Barnes et al. 2014 Original & Improved, Zhou et al. 2016, Wei et al. 2018), Priority-Flood+Epsilon, max-area depression filling, pit masks, watershed labeling, and `HasDepressions`.
  - **Depression breaching:** Lindsay (2016) Complete, Selective, and Constrained breaching (with optional epsilon gradients and hybrid filling).
  - **Depression Hierarchies & Fill-Spill-Merge:** Full `GetDepressionHierarchy` (Barnes et al. 2020) and `FillSpillMerge` (Barnes et al. 2021) hydrological water-routing engine.
  - **Flat resolution:** Barnes et al. (2014) flat mask, epsilon DEM alteration, D8 & D-infinity flat flow resolution, and Garbrecht & Martz (1997).
  - **Flow metrics & accumulation:** D8 / D4 (O'Callaghan & Mark 1984), Rho8 / Rho4 (Fairfield & Leymarie 1991), D-infinity (Tarboton 1997), Quinn (1991), Holmgren (1994), Freeman (1991), generic 3D flow-proportion accumulation, direct D8 & D-infinity accumulation, and ArcGIS / TauDEM D8 conversions.
  - **Terrain attributes:** Slope (rise/run, percentage, degrees, radians) and aspect (Horn 1981), total / planform / profile curvature (Zevenbergen & Thorne 1987), SPI, CTI, topographic surface area (Jenness 2004), and perimeter.
- **Zero C/C++ or GDAL dependencies:** Built on [`terrio`](https://github.com/opengeos/terrio) for pure-Rust reading and writing of GeoTIFF, Cloud Optimized GeoTIFF (COG), ESRI ASCII Grid, ENVI, HFA, VRT, and more — preserving geotransforms, CRS/projections, NoData values, and `PROCESSING_HISTORY` metadata.
- **Rust library, CLI (`richdem`), and Python bindings (`richdem-rs` via PyO3 + maturin).**

## Workspace Layout

| Path | Description |
|---|---|
| `crates/richdem` | Pure-Rust core library (`richdem`) |
| `crates/richdem-cli` | Standalone CLI (`richdem`) covering RichDEM's `rd_*` tools |
| `python` | Python bindings (`richdem-rs`, PyO3 + NumPy) with drop-in `pyRichDEM` API |
| `test_data` | Golden test fixtures from RichDEM (`depressions`, `breaching`, `flow_accum`) |

## Rust Usage

Add `richdem` to your `Cargo.toml`:

```toml
[dependencies]
richdem = { git = "https://github.com/opengeos/richdem-rs" }
```

```rust,no_run
use richdem::{
    Array2D, FlowAccumMethod, TerrainAttr, Topology,
    fill_depressions_epsilon, flow_accumulation_by_method,
    load_raster, save_raster, terrain_attribute,
};

fn main() -> richdem::Result<()> {
    // Load any GeoTIFF/COG/AAIGrid without GDAL (powered by terrio)
    let mut dem = load_raster::<f64>("dem.tif", None)?;

    // Fill depressions with an epsilon downhill gradient
    fill_depressions_epsilon(&mut dem, Topology::D8)?;
    save_raster(&dem, "dem_filled.tif", Some("fill_depressions_epsilon"), true)?;

    // Compute D-infinity flow accumulation
    let mut accum = Array2D::make_from_template(&dem, 1.0f64);
    flow_accumulation_by_method(&dem, &mut accum, FlowAccumMethod::Dinf, None)?;
    save_raster(&accum, "accum_dinf.tif", Some("flow_accumulation Dinf"), true)?;

    // Compute slope in degrees (Horn 1981)
    let slope = terrain_attribute(&dem, TerrainAttr::SlopeDegrees, 1.0);
    save_raster(&slope, "slope_deg.tif", Some("terrain_attribute slope_degrees"), true)?;
    Ok(())
}
```

## Command-Line Interface

Build and run the standalone `richdem` CLI:

```sh
cargo install --path crates/richdem-cli

richdem depressions-flood dem.tif filled.tif --gradient --compress
richdem depressions-breach dem.tif breached.tif --mode complete
richdem depressions-has dem.tif
richdem depressions-mask dem.tif pits.tif
richdem resolve-flats filled.tif resolved.tif
richdem d8-flowdirs resolved.tif flowdirs.tif
richdem flood-for-flowdirs dem.tif flowdirs.tif
richdem flow-accumulation resolved.tif accum.tif --method Dinf
richdem terrain-property dem.tif slope.tif --attrib slope_degrees
richdem surface-area dem.tif
richdem depression-hierarchy dem.tif
richdem fill-spill-merge dem.tif wtd.tif --initial-water 1.0
richdem info filled.tif
richdem compare dem.tif filled.tif
```

## Python Usage

Build locally with [maturin](https://www.maturin.rs/):

```sh
cd python
maturin develop --release
```

```python
import richdem_rs as rd

dem = rd.LoadGDAL("dem.tif")  # Pure-Rust via terrio, no GDAL required!
filled = rd.FillDepressions(dem, epsilon=True)
accum = rd.FlowAccumulation(filled, method="Dinf")
slope = rd.TerrainAttribute(filled, attrib="slope_degrees")
rd.SaveGDAL("accum.tif", accum, compress=True)
```

## Testing

```sh
cargo test --workspace
```

All golden fixtures from RichDEM (`test_data/depressions/`, `test_data/breaching/`, `test_data/flow_accum/`, `test_data/ones_block.dem`) and the full Fill-Spill-Merge unit and randomized property test suite (heavy flooding vs Priority-Flood, idempotence, incremental vs big dump, and mass conservation) are verified in `crates/richdem/tests/`.

## Publications & Credits

Please cite [**RichDEM**](https://github.com/r-barnes/richdem) and the relevant algorithm publications when using this software:

- **RichDEM:** Barnes, R., 2016. *RichDEM: Terrain Analysis Software*. <https://github.com/r-barnes/richdem>
- **Priority-Flood:** Barnes, R., Lehman, C., Mulla, D., 2014. Priority-flood: An optimal depression-filling and watershed-labeling algorithm for digital elevation models. *Computers & Geosciences* 62, 117-127. doi:[10.1016/j.cageo.2013.04.024](https://doi.org/10.1016/j.cageo.2013.04.024)
- **Flat Resolution:** Barnes, R., Lehman, C., Mulla, D., 2014. An efficient assignment of drainage direction over flat surfaces in raster digital elevation models. *Computers & Geosciences* 62, 128-135. doi:[10.1016/j.cageo.2013.01.009](https://doi.org/10.1016/j.cageo.2013.01.009)
- **Depression Hierarchies:** Barnes, R., Callaghan, K.L., Wickert, A.D., 2020. Computing water flow through complex landscapes - Part 2: Finding hierarchies in depressions and morphological segmentations. *Earth Surface Dynamics* 8, 431-445. doi:[10.5194/esurf-8-431-2020](https://doi.org/10.5194/esurf-8-431-2020)
- **Fill-Spill-Merge:** Barnes, R., Callaghan, K.L., Wickert, A.D., 2021. Computing water flow through complex landscapes - Part 3: Fill-Spill-Merge: Flow routing in depression hierarchies. *Earth Surface Dynamics* 9, 105-121. doi:[10.5194/esurf-9-105-2021](https://doi.org/10.5194/esurf-9-105-2021)
- **Zhou2016 Priority-Flood:** Zhou, G., Sun, Z., Fu, S., 2016. An efficient variant of the Priority-Flood algorithm for filling depressions in raster digital elevation models. *Computers & Geosciences* 90, 87-96. doi:[10.1016/j.cageo.2016.02.021](https://doi.org/10.1016/j.cageo.2016.02.021)
- **Wei2018 Priority-Flood:** Wei, H., Zhou, G., Fu, S., 2018. Efficient Priority-Flood depression filling in raster digital elevation models. *International Journal of Digital Earth* 12, 415-427. doi:[10.1080/17538947.2018.1429503](https://doi.org/10.1080/17538947.2018.1429503)
- **Depression Breaching:** Lindsay, J.B., 2016. Efficient hybrid breaching-filling sink removal methods for flow path enforcement in digital elevation models. *Hydrological Processes* 30, 846-857. doi:[10.1002/hyp.10648](https://doi.org/10.1002/hyp.10648)
- **D-Infinity:** Tarboton, D.G., 1997. A new method for the determination of flow directions and upslope areas in grid digital elevation models. *Water Resources Research* 33, 309-319.
- **D8 / D4:** O'Callaghan, J.F., Mark, D.M., 1984. The extraction of drainage networks from digital elevation data. *CVGIP* 28, 323-344.
- **Rho8 / Rho4:** Fairfield, J., Leymarie, P., 1991. Drainage networks from grid digital elevation models. *Water Resources Research* 27, 709-717.
- **MFD:** Quinn et al. (1991); Freeman (1991); Holmgren (1994).
- **Slope / Aspect / Curvature / Surface Area:** Horn (1981); Zevenbergen & Thorne (1987); Garbrecht & Martz (1997); Jenness (2004).
- **Raster I/O:** [`terrio`](https://github.com/opengeos/terrio) (MIT / MPL-2.0 / Apache-2.0).

See [NOTICE](NOTICE) for full third-party attribution.

## License

`richdem-rs` is licensed under the **GNU General Public License v3.0 (`GPL-3.0-only`)**, matching the upstream [RichDEM](https://github.com/r-barnes/richdem) license. See [LICENSE](LICENSE) for the full text.
