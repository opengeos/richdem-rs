# Installation

`richdem-rs` ships as a Python package on PyPI, Rust crates on crates.io, and standalone prebuilt CLI binaries on GitHub Releases — all with zero C/C++ or GDAL system dependencies.

## Python (`richdem-rs`)

Install prebuilt wheels from [PyPI](https://pypi.org/project/richdem-rs/) (Python 3.11–3.14 on Linux, macOS, and Windows):

```sh
pip install richdem-rs
```

Or with [`uv`](https://docs.astral.sh/uv/):

```sh
uv pip install richdem-rs
```

For plotting with `rdShow`, install `matplotlib` as well:

```sh
pip install richdem-rs matplotlib
```

### Building the Python package from source

Requires Rust >= 1.90 and [maturin](https://www.maturin.rs/):

```sh
git clone https://github.com/opengeos/richdem-rs.git
cd richdem-rs/python
pip install maturin numpy
maturin develop --release
```

## Rust Library (`richdem`)

Add `richdem` to your `Cargo.toml`:

```sh
cargo add richdem
```

Or manually:

```toml
[dependencies]
richdem = "0.1"
```

### Cargo Features

| Feature | Default | Description |
|---|---|---|
| `terrio` | Yes | Pure-Rust geospatial raster I/O (GeoTIFF, COG, AAIGrid, ENVI, HFA, VRT) via [`terrio`](https://terrio.dev) |
| `parallel` | Yes | Multi-threaded row-parallel execution via `rayon` |
| `serde` | No | `Serialize` / `Deserialize` support for `Array2D` and `DepressionHierarchy` |

To compile without raster I/O or parallelism (e.g., for embedded or single-threaded builds):

```toml
[dependencies]
richdem = { version = "0.1", default-features = false }
```

## Command-Line Interface (`richdem`)

### From crates.io

```sh
cargo install richdem-cli
```

### Prebuilt binaries

Download prebuilt archives for Linux (`x86_64`, `aarch64`), macOS (`x86_64`, `aarch64`), and Windows (`x86_64`, `aarch64`) from [GitHub Releases](https://github.com/opengeos/richdem-rs/releases).

Verify the installation:

```sh
richdem --version
richdem --help
```
