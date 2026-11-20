# Rust Library Reference

Add `richdem` to your project:

```sh
cargo add richdem
```

Full API rustdocs are published automatically on [docs.rs/richdem](https://docs.rs/richdem).

## Key Types

- **`Array2D<T: DemNum>`**: Generic 2D raster storing row-major data, `no_data`, `geotransform`, `projection`, and `metadata`. Supports `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, and `f64`.
- **`DynamicArray2D`**: Runtime-typed enum wrapping `Array2D<T>` across all 10 numeric types for generic CLI/I/O workflows.
- **`Array3D<f32>`**: 3D `(width, height, 9)` flow proportion array (`FLOW_PROPS_DEPTH = 9`).
- **`Topology`**: `Topology::D8` or `Topology::D4`.
- **`FlowAccumMethod`**: `D8`, `D4`, `Tarboton` (`Dinf`), `Quinn`, `Holmgren`, `Freeman`, `Rho8`, `Rho4`.
- **`TerrainAttr`**: `SlopeRiseRun`, `SlopePercentage`, `SlopeDegrees`, `SlopeRadians`, `Aspect`, `Curvature`, `PlanformCurvature`, `ProfileCurvature`.
- **`DepressionHierarchy<T>` / `Depression<T>`**: Binary hierarchy of nested topographic depressions.
- **`Error` / `Result<T>`**: Structured error type via `thiserror`.

## Module Overview

| Module | Contents |
|---|---|
| `array2d` / `array3d` | `Array2D`, `DynamicArray2D`, `Array3D`, geotransform helpers, parallel row helpers |
| `depressions` | Priority-Flood (`wei2018`, `zhou2016`, `barnes2014`, `original`, `epsilon`, `max_dep`, `flowdirs`, `watersheds`), `lindsay2016` breaching, `has_depressions`, `pit_mask`, `get_depression_hierarchy`, `fill_spill_merge` |
| `flats` | `find_flats`, `get_flat_mask`, `resolve_flats_epsilon`, `barnes_flat_resolution_d8`, `resolve_flats_barnes_dinf`, `garbrecht_alg` |
| `flowmet` | `d8_flow_directions`, `dinf_flow_directions`, `fm_ocallaghan`, `fm_tarboton`, `fm_quinn`, `fm_holmgren`, `fm_freeman`, `fm_fairfield_leymarie` |
| `methods` | `flow_accumulation`, `flow_accumulation_by_method`, `flow_proportions_by_method`, `d8_flow_accum`, `dinf_upslope_area`, `terrain_attribute`, `ta_slope_*`, `ta_aspect`, `ta_*curvature`, `ta_spi`, `ta_cti` |
| `misc` | `dem_surface_area`, `perimeter`, `bucket_fill`, `find_flow_loops`, ArcGIS / TauDEM D8 conversions |
| `io` *(feature `terrio`)* | `load_raster`, `load_raster_padded`, `load_dynamic`, `save_raster`, `save_dynamic`, `array2d_to_dataset`, `dataset_to_dynamic` |
| `terrain_gen` | `PerlinNoise`, `generate_perlin_terrain`, `try_generate_perlin_terrain` |

## Example: Custom Feature Build

```toml
[dependencies]
# Core algorithms + rayon parallelism, without raster I/O
richdem = { version = "0.2", default-features = false, features = ["parallel"] }
```

```rust
use richdem::{
    Array2D, FlowAccumMethod, TerrainAttr, Topology,
    fill_depressions_epsilon, flow_accumulation_by_method,
    generate_perlin_terrain, terrain_attribute,
};

fn main() -> richdem::Result<()> {
    let mut dem = generate_perlin_terrain(512, 42);
    fill_depressions_epsilon(&mut dem, Topology::D8)?;

    let mut accum = Array2D::make_from_template(&dem, 1.0f64);
    flow_accumulation_by_method(&dem, &mut accum, FlowAccumMethod::Quinn, None)?;
    let slope = terrain_attribute(&dem, TerrainAttr::SlopeDegrees, 1.0);

    println!("Grid: {}x{}, slope[10,10]={:.2}", slope.width(), slope.height(), slope.get(10, 10));
    Ok(())
}
```
