# richdem-rs contributor instructions

## Scope and architecture

`richdem-rs` is a pure-Rust reimplementation of Richard Barnes's [RichDEM](https://github.com/r-barnes/richdem) under `GPL-3.0-only`, using [`terrio`](https://github.com/opengeos/terrio) for GDAL-free geospatial raster I/O.

- Put all hydrologic algorithms and raster structures in `crates/richdem`; keep `crates/richdem-cli` and `python/` thin wrappers.
- Keep default builds 100% pure Rust with zero C/C++ or GDAL dependencies.
- Maintain credit to Richard Barnes, upstream RichDEM, and original algorithm authors in doc comments, `NOTICE`, and `CITATION.cff`. Keep `python/LICENSE` and `python/NOTICE` synchronized with the root `LICENSE` and `NOTICE` so sdist and wheel builds include them.
- Preserve upstream RichDEM golden test fixtures under `test_data/` verbatim (they are excluded from whitespace/EOF fixers in `.pre-commit-config.yaml`).

## Invariants and gotchas

- **NoData and NaN safety:** Every algorithm must preserve NoData cells without modifying their values, treating non-finite float elevations as missing where appropriate. Use `DemNum::rd_eq` / `rd_cmp` so `NaN` comparisons behave consistently.
- **Exact integer conversions:** Keep integer-to-integer pixel conversions (`DemNum::cast_to`, `array2d_to_dataset`, `SaveGDAL`) off the `f64` path so `u64`/`i64` pixel values above `2^53` round-trip losslessly (note: `terrio::RasterConfig.nodata` is typed `f64` upstream, so NoData metadata sentinels above `2^53` are bounded by `f64` precision). Python hydrologic wrappers (`FillDepressions`, `BreachDepressions`, `FlowAccumulation`, `TerrainAttribute`, etc.) convert input DEMs to `f64` (or `f32` for `float32` epsilon filling) to match upstream `pyRichDEM` and accept default NumPy `int64` arrays.
- **D-infinity no-flow sentinel:** Use `DINF_NO_FLOW` (`-2.0`) rather than `NO_FLOW` (`0`) for flat/pit cells in D-infinity angle rasters; `0.0` is valid eastward flow.
- **Checked raster sizing:** Use `checked_mul` for all `width * height` (and `width * height * 9`) calculations and cap preallocation when parsing untrusted headers.
- **Python packaging (`python/`):** `python/` is excluded from the root Cargo workspace so pure-Rust builds never require `libpython`. Minimum supported Python is **3.11** (`requires-python = ">=3.11"`, `abi3-py311`, `[tool.ruff] target-version = "py311"`). Always build Python bindings inside a virtualenv (`maturin develop --release`).

## Validation

Run the full gate matching `.github/workflows/ci.yml` before pushing:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p richdem --no-default-features --lib
cargo test --workspace --all-features
```

Run `cargo audit` from the repository root so it reads `.cargo/audit.toml` (each ignore carries a justification, and each ignore group states a removal condition; CI's `audit` job runs the same checks); audit the separate `python/` and `js/` workspaces from the root too, with `cargo audit -f python/Cargo.lock` and `cargo audit -f js/Cargo.lock`. Only `js/Cargo.lock` is tracked in git, so generate the others on a fresh clone first:

```sh
cargo generate-lockfile
cargo generate-lockfile --manifest-path python/Cargo.toml
cargo audit && cargo audit -f python/Cargo.lock && cargo audit -f js/Cargo.lock
```

When touching `python/` or `scripts/`, also rebuild and test the Python bindings and verify sdist packaging if metadata/manifests changed:

```sh
cd python && .venv/bin/maturin develop --release && .venv/bin/pytest -v
```

Run `pre-commit run --files <changed-files>` on modified files before committing. Add regression tests in `crates/richdem/tests/` or `python/tests/` for every bug fix, and report tolerances explicitly rather than calling approximate agreement identical.

## GitHub workflow

- Submit all changes through feature branches and pull requests; never push implementation commits directly to `main`.
- Batch verified review fixes into one commit per round, reply to each review comment with the commit SHA and a one-line summary, and resolve addressed threads.
- Keep `README.md` benchmarks reproducible via `cargo run --release --example benchmark` and `scripts/benchmark.py`; update published numbers only from fresh `--release` runs.
