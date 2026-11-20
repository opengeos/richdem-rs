# richdem-rs

[![CI](https://github.com/opengeos/richdem-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/opengeos/richdem-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/richdem.svg)](https://crates.io/crates/richdem)
[![PyPI](https://img.shields.io/pypi/v/richdem-rs.svg)](https://pypi.org/project/richdem-rs/)
[![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/opengeos/richdem-rs/blob/main/examples/richdem_rs_demo.ipynb)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://github.com/opengeos/richdem-rs/blob/main/LICENSE)

**High-performance Digital Elevation Model (DEM) hydrologic and terrain analysis in pure Rust**, with GDAL-free geospatial raster I/O powered by [`terrio`](https://terrio.dev).

!!! note "Credits & Attribution"
    `richdem-rs` is a pure-Rust reimplementation of [**RichDEM**](https://github.com/r-barnes/richdem), originally designed, authored, and published by [**Richard Barnes**](https://github.com/r-barnes) (`rbarnes@umn.edu`). All credit for the library design, hydrologic algorithms, data structures, and test suite belongs to Richard Barnes and the original algorithm authors listed under [Publications & Credits](credits.md).

    **Disclaimer:** This repository was created using **Google Gemini 4 Neon**.

## Quick Taste

=== "Python"

    ```python
    import richdem_rs as rd

    dem = rd.LoadGDAL("dem.tif")  # Pure-Rust via terrio — no GDAL install required
    filled = rd.FillDepressions(dem, epsilon=True)
    accum = rd.FlowAccumulation(filled, method="Dinf")
    slope = rd.TerrainAttribute(filled, attrib="slope_degrees")
    rd.SaveGDAL("accum.tif", accum, compress=True)
    ```

=== "Rust"

    ```rust
    use richdem::{
        Array2D, FlowAccumMethod, TerrainAttr, Topology,
        fill_depressions_epsilon, flow_accumulation_by_method,
        load_raster, save_raster, terrain_attribute,
    };

    fn main() -> richdem::Result<()> {
        let mut dem = load_raster::<f64>("dem.tif", None)?;
        fill_depressions_epsilon(&mut dem, Topology::D8)?;

        let mut accum = Array2D::make_from_template(&dem, 1.0f64);
        flow_accumulation_by_method(&dem, &mut accum, FlowAccumMethod::Dinf, None)?;
        let slope = terrain_attribute(&dem, TerrainAttr::SlopeDegrees, 1.0);

        save_raster(&accum, "accum.tif", Some("flow_accumulation Dinf"), true)?;
        save_raster(&slope, "slope_deg.tif", Some("terrain_attribute slope_degrees"), true)?;
        Ok(())
    }
    ```

=== "CLI"

    ```sh
    richdem depressions-flood dem.tif filled.tif --gradient --compress
    richdem flow-accumulation filled.tif accum.tif --method Dinf --compress
    richdem terrain-property filled.tif slope.tif --attrib slope_degrees --compress
    ```

## Highlights

- **Full algorithmic parity with RichDEM:**
  - **Depression filling:** Priority-Flood (Barnes et al. 2014 Original & Improved, Zhou et al. 2016, Wei et al. 2018), Priority-Flood+Epsilon, max-area filling, pit masks, watershed labeling, and `HasDepressions`. See [Depressions & Breaching](depressions.md).
  - **Depression breaching:** Lindsay (2016) Complete, Selective, and Constrained breaching with optional epsilon gradients and hybrid filling.
  - **Depression Hierarchies & Fill-Spill-Merge:** Full `GetDepressionHierarchy` (Barnes et al. 2020) and `FillSpillMerge` (Barnes et al. 2021) water-routing engine. See [Depression Hierarchies & FSM](depression-hierarchy.md).
  - **Flat resolution:** Barnes et al. (2014) flat mask, epsilon DEM alteration, D8 & D-infinity flat flow resolution, and Garbrecht & Martz (1997). See [Flats & Flow Routing](flow-routing.md).
  - **Flow metrics & accumulation:** D8 / D4 (O'Callaghan & Mark 1984), Rho8 / Rho4 (Fairfield & Leymarie 1991), D-infinity (Tarboton 1997), Quinn (1991), Holmgren (1994), Freeman (1991), generic 3D flow-proportion accumulation, direct D8 & D-infinity accumulation, and ArcGIS / TauDEM D8 conversions.
  - **Terrain attributes:** Slope (rise/run, percentage, degrees, radians) and aspect (Horn 1981), total / planform / profile curvature (Zevenbergen & Thorne 1987), SPI, CTI, topographic surface area (Jenness 2004), and perimeter. See [Terrain Attributes](terrain-attributes.md).
- **Zero C/C++ or GDAL dependencies:** Built on [`terrio`](https://terrio.dev) for pure-Rust GeoTIFF, COG, AAIGrid, ENVI, HFA, and VRT reading and writing — preserving geotransforms, CRS/projections, NoData values, and `PROCESSING_HISTORY` metadata.
- **Native multi-core scaling:** Pure-Rust `rayon` parallelism enabled by default (no OpenMP or C runtime required). See [Multithreading](parallelism.md).
- **Interactive Colab demo:** Try `richdem-rs` in your browser with no local setup via the [Google Colab notebook](https://colab.research.google.com/github/opengeos/richdem-rs/blob/main/examples/richdem_rs_demo.ipynb).

## Where to Go Next

| You want to | Read |
|---|---|
| Install the Python package, Rust crate, or CLI | [Installation](installation.md) |
| Run your first hydrologic workflow | [Getting Started](getting-started.md) |
| Migrate existing `richdem` / `richdem2` scripts | [Coming from RichDEM](coming-from-richdem.md) |
| Look up a Python, Rust, or CLI API | [Python](python.md), [Rust Library](rust.md), [Command Line](cli.md) |
| Compare performance and multi-core scaling | [Benchmarks](benchmarks.md), [Multithreading](parallelism.md) |
| Cite RichDEM and algorithm papers | [Publications & Credits](credits.md) |
