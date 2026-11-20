# Flats & Flow Routing

After depression filling, large flat regions may remain with zero local gradient. `richdem-rs` implements Barnes et al. (2014) and Garbrecht & Martz (1997) flat resolution alongside RichDEM's full family of single- and multiple-flow-direction metrics.

## Flat Resolution

| Method | Rust Function | Behavior |
|---|---|---|
| Barnes (2014) Epsilon Alteration | `resolve_flats_epsilon` / `resolve_flats_epsilon_barnes2014` | Combines away-from-higher and towards-lower gradients and applies minimal `next_up` floating-point increments in place |
| Barnes (2014) Flat Mask | `get_flat_mask` / `resolve_flats_barnes` | Returns `(flat_mask, labels)` without altering the DEM |
| Barnes (2014) D8 Flow Directions | `barnes_flat_resolution_d8` / `d8_flow_flats` | Resolves D8 directions directly over flats |
| Barnes (2014) D-Infinity Flow Directions | `resolve_flats_barnes_dinf` / `dinf_flow_flats` | Couples Barnes (2014) flat masks with Tarboton (1997) D-infinity angles |
| Garbrecht & Martz (1997) | `garbrecht_alg` | Classic iterative away/towards flat gradient algorithm |
| Flat Cell Detection | `find_flats` | Labels flat cells (`IS_A_FLAT`, `NOT_A_FLAT`, `FLAT_NO_DATA`) |

```python
import richdem_rs as rd

filled = rd.FillDepressions(dem, epsilon=False)
resolved = rd.ResolveFlats(filled)
```

## Flow Accumulation Methods

Pass any of the following method names to `FlowAccumulation` (Python), `FlowAccumMethod::parse` (Rust), or `--method` (CLI):

| Method Name | Aliases | Author(s) | Type | Exponent |
|---|---|---|---|---|
| `D8` | `OCallaghan`, `OCallaghanD8` | O'Callaghan & Mark (1984) | Single-flow (8-way) | — |
| `D4` | `OCallaghanD4` | O'Callaghan & Mark (1984) | Single-flow (4-way) | — |
| `Dinf` | `Tarboton`, `Dinfinity` | Tarboton (1997) | Continuous angle (2-way split) | — |
| `Quinn` | — | Quinn et al. (1991) | Multiple-flow (MFD, linear) | — |
| `Holmgren` | — | Holmgren (1994) | Multiple-flow (MFD, power) | Required (`x > 0`, e.g. `4.0`) |
| `Freeman` | — | Freeman (1991) | Multiple-flow (MFD, power) | Required (`x > 0`, e.g. `1.1`) |
| `Rho8` | `FairfieldLeymarieD8`, `FairfieldLeymarie` | Fairfield & Leymarie (1991) | Stochastic single-flow (8-way) | — |
| `Rho4` | `FairfieldLeymarieD4` | Fairfield & Leymarie (1991) | Stochastic single-flow (4-way) | — |

### Two-Stage Proportions & Custom Weights

For repeated accumulation runs with different runoff or rainfall weights on the same DEM, compute the `(H, W, 9)` flow proportions once and reuse them:

```python
import numpy as np
import richdem_rs as rd

props = rd.FlowProportions(filled, method="Holmgren", exponent=4.0)
uniform_accum = rd.FlowAccumFromProps(props)

weights = rd.rdarray(np.full(filled.shape, 2.5), meta_obj=filled, no_data=-1)
weighted_accum = rd.FlowAccumFromProps(props, weights=weights)
```

### Flow Direction Conversions & Loop Checking

- `convert_arc_flowdirs_to_richdem_d8` (`richdem arc-d8-to-richdem-d8`): converts ArcGIS D8 powers-of-two (`1, 2, 4, ..., 128`) to RichDEM D8 codes (`1..=8`).
- `convert_taudem_d8_to_richdem_d8` (`richdem taudem-d8-to-richdem-d8`): converts TauDEM D8 codes (`1..=8`) to RichDEM D8 codes.
- `find_flow_loops` (`richdem loop-check`): detects cycles in a D8 flow direction raster.
