# Depressions & Breaching

Digital Elevation Models frequently contain topographic depressions (pits and sinks) caused by measurement noise, resampling artifacts, or real closed basins. `richdem-rs` implements RichDEM's full suite of depression detection, filling, and breaching algorithms.

## Depression Filling (Priority-Flood)

All Priority-Flood variants preserve NoData cells and support both `D8` (8-neighbor) and `D4` (4-neighbor) topologies where applicable.

| Algorithm | Rust Function | Complexity / Notes |
|---|---|---|
| Wei et al. (2018) | `priority_flood_wei2018` | Fastest Priority-Flood in [Benchmarks](benchmarks.md) (52.2 M cells/s); called directly |
| Zhou et al. (2016) | `priority_flood_zhou2016` | High-speed Priority-Flood minimizing heap pushes (default behind `fill_depressions` for D8) |
| Barnes et al. (2014) Improved | `priority_flood_barnes2014` | Classic FIFO-pit + priority-queue Priority-Flood (default behind `fill_depressions` for D4) |
| Barnes et al. (2014) Original | `priority_flood_original` | Reference Priority-Flood without FIFO pit queue |
| Priority-Flood + Epsilon | `priority_flood_epsilon_barnes2014` / `fill_depressions_epsilon` | Imposes minimal `next_up` floating-point downhill gradients across filled flats (floating-point DEMs only) |
| Max-Area Filling | `priority_flood_barnes2014_max_dep` | Fills only depressions below a maximum cell count |
| Implicit Flow Directions | `priority_flood_flowdirs_barnes2014` | Computes D8 flow directions across depressions without modifying the DEM |
| Watershed Labeling | `priority_flood_watersheds_barnes2014` | Assigns watershed labels during Priority-Flood |

### Python

```python
import richdem_rs as rd

# Fill depressions (Zhou 2016 for D8, Barnes 2014 for D4)
filled = rd.FillDepressions(dem, epsilon=False, topology="D8")

# Fill with minimal downhill epsilon gradients (recommended before flow routing)
filled_eps = rd.FillDepressions(dem, epsilon=True, topology="D8")
```

### CLI

```sh
richdem depressions-flood dem.tif filled.tif --gradient --topology d8 --compress
```

## Depression Breaching (Lindsay 2016)

Depression breaching carves minimal drainage channels through enclosing sills rather than raising pit elevations, often producing more realistic flow paths across road embankments and narrow ridges.

| Mode | Rust API | CLI Flag |
|---|---|---|
| Complete Breaching | `breach_depressions` / `complete_breaching_lindsay2016` | `--mode complete` |
| Selective Breaching | `lindsay2016(..., LindsayMode::SelectiveBreaching, ...)` | `--mode selective --max-length N --max-depth D` |
| Constrained Breaching | `lindsay2016(..., LindsayMode::ConstrainedBreaching, ...)` | `--mode constrained --max-length N --max-depth D` |
| Hybrid Breach + Fill | `lindsay2016(..., fill_depressions = true, ...)` | `--fill-depressions` |

### Python

```python
breached = rd.BreachDepressions(dem, topology="D8")
```

### CLI

```sh
# Complete breaching
richdem depressions-breach dem.tif breached.tif --mode complete --compress

# Constrained hybrid breaching + filling with epsilon gradients
richdem depressions-breach dem.tif hybrid.tif \
  --mode constrained --max-length 25 --max-depth 5.0 \
  --fill-depressions --gradient --compress
```

## Depression Detection & Pit Masks

- **`has_depressions`** (`richdem depressions-has dem.tif`): returns `true` if any interior valid cell has no strictly lower neighbor.
- **`pit_mask`** (`richdem depressions-mask dem.tif pits.tif`): outputs a `u8` mask (`1` = pit, `0` = non-pit, `3` = NoData).
