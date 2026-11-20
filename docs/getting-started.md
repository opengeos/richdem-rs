# Getting Started

This walkthrough covers a complete hydrologic conditioning and terrain analysis pipeline: loading (or generating) a DEM, removing depressions, routing flow, computing terrain attributes, and saving results to GeoTIFF.

## 1. Python Workflow

```python
import numpy as np
import richdem_rs as rd

# Load a GeoTIFF/COG/AAIGrid directly via pure-Rust terrio (no GDAL needed)
# Or generate a synthetic Perlin-noise DEM to experiment immediately:
dem = rd.generate_perlin_terrain(256, seed=42)

# Fill depressions with an epsilon downhill gradient (Barnes et al. 2014)
filled = rd.FillDepressions(dem, epsilon=True)

# Alternative: breach depressions (Lindsay 2016)
breached = rd.BreachDepressions(dem)

# Compute flow accumulation (D8, Dinf, Quinn, Holmgren, Freeman, Rho8, ...)
accum_dinf = rd.FlowAccumulation(filled, method="Dinf")
accum_d8 = rd.FlowAccumulation(filled, method="D8")

# Compute slope and curvature (Horn 1981 / Zevenbergen & Thorne 1987)
slope_deg = rd.TerrainAttribute(filled, attrib="slope_degrees")
curv = rd.TerrainAttribute(filled, attrib="curvature")

# Save outputs as compressed GeoTIFFs with metadata & PROCESSING_HISTORY
rd.SaveGDAL("filled.tif", filled, compress=True)
rd.SaveGDAL("accum_dinf.tif", accum_dinf, compress=True)
rd.SaveGDAL("slope_deg.tif", slope_deg, compress=True)

print("Max Dinf accumulation:", float(np.max(accum_dinf)))
print("Processing history:\n", accum_dinf.metadata.get("PROCESSING_HISTORY", ""))
```

You can run this full interactive workflow (including Fill-Spill-Merge and visualizations) directly in your browser using the [Google Colab demo notebook](https://colab.research.google.com/github/opengeos/richdem-rs/blob/main/examples/richdem_rs_demo.ipynb).

## 2. Rust Workflow

```rust
use richdem::{
    Array2D, FlowAccumMethod, TerrainAttr, Topology,
    fill_depressions_epsilon, flow_accumulation_by_method,
    generate_perlin_terrain, save_raster, terrain_attribute,
};

fn main() -> richdem::Result<()> {
    // Generate a 256x256 synthetic DEM (or use load_raster::<f64>("dem.tif", None)?)
    let mut dem = generate_perlin_terrain(256, 42);

    // Fill depressions with an epsilon downhill gradient
    fill_depressions_epsilon(&mut dem, Topology::D8)?;

    // Compute D-infinity flow accumulation
    let mut accum = Array2D::make_from_template(&dem, 1.0f64);
    flow_accumulation_by_method(&dem, &mut accum, FlowAccumMethod::Dinf, None)?;

    // Compute Horn (1981) slope in degrees
    let slope = terrain_attribute(&dem, TerrainAttr::SlopeDegrees, 1.0);

    // Save compressed GeoTIFFs with PROCESSING_HISTORY
    save_raster(&dem, "filled.tif", Some("fill_depressions_epsilon"), true)?;
    save_raster(&accum, "accum_dinf.tif", Some("flow_accumulation Dinf"), true)?;
    save_raster(&slope, "slope_deg.tif", Some("terrain_attribute slope_degrees"), true)?;
    Ok(())
}
```

## 3. Command-Line Workflow

```sh
# 1. Inspect input DEM metadata
richdem info dem.tif

# 2. Fill depressions with an epsilon gradient
richdem depressions-flood dem.tif filled.tif --gradient --compress

# 3. Compute D-infinity flow accumulation
richdem flow-accumulation filled.tif accum.tif --method Dinf --compress

# 4. Compute slope in degrees
richdem terrain-property filled.tif slope.tif --attrib slope_degrees --compress

# 5. Check topographic surface area
richdem surface-area filled.tif
```
