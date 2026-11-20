# Depression Hierarchies & Fill-Spill-Merge

`richdem-rs` includes the full implementation of:

- **Depression Hierarchies** (Barnes, Callaghan, & Wickert, 2020) — builds a binary tree capturing every nested topographic depression, its pit cell, sill (outlet) cell, marginal/total volume, and ocean connectivity.
- **Fill-Spill-Merge** (Barnes, Callaghan, & Wickert, 2021) — routes surface water through the depression hierarchy in `O(N log N)` time while strictly conserving mass, filling depressions to their sill levels, spilling excess water downstream, and merging flooded sub-basins.

## Python Example

```python
import numpy as np
import richdem_rs as rd

dem = rd.generate_perlin_terrain(128, seed=42)

# 1. Initialize labels with OCEAN on borders and NO_DEP in the interior
labels = rd.get_new_depression_hierarchy_labels(dem.shape, geotransform=dem.geotransform)

# 2. Construct the depression hierarchy (updates `labels` in place)
deps, flowdirs = rd.get_depression_hierarchy(dem, labels, topology="D8")
print(f"Found {len(deps)} depressions (including ocean node 0)")

# 3. Route uniform initial surface water depth using Fill-Spill-Merge
wtd = rd.rdarray(np.full(dem.shape, 0.25, dtype=np.float64), meta_obj=dem, no_data=-9999)
rd.fill_spill_merge(dem, labels, flowdirs, deps, wtd)

print(f"Max ponded water depth: {float(np.max(wtd)):.3f}")
```

## Rust Example

```rust
use richdem::{
    Array2D, NO_DEP, NO_FLOW, OCEAN, Topology,
    fill_spill_merge, generate_perlin_terrain, get_depression_hierarchy,
};

fn main() -> richdem::Result<()> {
    let dem = generate_perlin_terrain(128, 42);
    let mut labels = Array2D::make_from_template(&dem, NO_DEP);
    labels.set_edges(OCEAN);
    let mut flowdirs = Array2D::make_from_template(&dem, NO_FLOW);

    let mut dh = get_depression_hierarchy(&dem, &mut labels, &mut flowdirs, Topology::D8)?;
    let mut wtd = Array2D::make_from_template(&dem, 0.25f64);

    fill_spill_merge(&dem, &labels, &flowdirs, &mut dh, &mut wtd)?;
    println!("Depressions: {}", dh.len());
    Ok(())
}
```

## CLI Example

```sh
# Print the depression hierarchy tree and optionally export labels & flowdirs
richdem depression-hierarchy dem.tif --labels-out labels.tif --flowdirs-out fds.tif

# Run Fill-Spill-Merge with uniform initial surface water depth (or --wtd-in raster)
richdem fill-spill-merge dem.tif wtd_out.tif --initial-water 0.5 --compress
```

## Verification

`crates/richdem/tests/fsm_tests.rs` verifies Fill-Spill-Merge against RichDEM's full property test suite:

- Heavy flooding matches Priority-Flood elevation surfaces.
- Repeated Fill-Spill-Merge applications are idempotent.
- Incremental water additions match a single equivalent dump.
- Total water volume is conserved across all interior depressions and ocean outflow.
