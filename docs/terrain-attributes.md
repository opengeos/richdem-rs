# Terrain Attributes

`richdem-rs` computes first- and second-order topographic derivatives using the raster's `geotransform` cell spacing (`dx`, `dy`) and an optional vertical `zscale` factor, parallelized across rows with `rayon`.

## Supported Attributes (`TerrainAttribute` / `terrain_attribute`)

| Attribute Name | Rust Enum (`TerrainAttr`) | Algorithm / Units |
|---|---|---|
| `slope_riserun` | `SlopeRiseRun` | Horn (1981) rise / run (dimensionless) |
| `slope_percentage` | `SlopePercentage` | Horn (1981) percent slope (`100 × rise / run`) |
| `slope_degrees` | `SlopeDegrees` | Horn (1981) slope in degrees (`0°–90°`) |
| `slope_radians` | `SlopeRadians` | Horn (1981) slope in radians (`0–π/2`) |
| `aspect` | `Aspect` | Horn (1981) compass aspect in degrees (`0°–360°`, `-1` on flats) |
| `curvature` | `Curvature` | Zevenbergen & Thorne (1987) total curvature |
| `planform_curvature` | `PlanformCurvature` | Zevenbergen & Thorne (1987) planform (contour) curvature |
| `profile_curvature` | `ProfileCurvature` | Zevenbergen & Thorne (1987) profile (slope-line) curvature |

### Python

```python
import richdem_rs as rd

slope_deg = rd.TerrainAttribute(dem, attrib="slope_degrees", zscale=1.0)
aspect = rd.TerrainAttribute(dem, attrib="aspect")
prof_curv = rd.TerrainAttribute(dem, attrib="profile_curvature")
plan_curv = rd.TerrainAttribute(dem, attrib="planform_curvature")
```

### CLI

```sh
richdem terrain-property dem.tif slope.tif --attrib slope_degrees --zscale 1.0 --compress
richdem terrain-property dem.tif aspect.tif --attrib aspect --compress
richdem terrain-property dem.tif curv.tif --attrib curvature --compress
```

## Hydrologic Indices & Surface Metrics

- **Stream Power Index (`ta_spi`)**: `ln((A / cell_area) * (S + 0.001))` from flow accumulation `A` and rise/run slope `S`.
- **Compound Topographic Index / Topographic Wetness Index (`ta_cti`)**: `ln((A / cell_area) / (S + 0.001))` from flow accumulation `A` and rise/run slope `S`.
- **Topographic Surface Area (`dem_surface_area` / `richdem surface-area`)**: Jenness (2004) 8-triangle surface area around every valid cell, using deterministic per-row partial sums so parallel and serial builds produce identical results.
- **Perimeter (`perimeter`)**: `PerimType::CellCount` (border cells) or `PerimType::SquareEdge` (sum of exposed horizontal and vertical edge lengths).
