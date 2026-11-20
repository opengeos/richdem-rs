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
python -m venv .venv
source .venv/bin/activate
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
richdem = "0.2"
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
richdem = { version = "0.2", default-features = false }
```

## Command-Line Interface (`richdem`)

### Shell / PowerShell installer (recommended)

The installer downloads the standalone `richdem` binary for your platform (Linux `x86_64` and `aarch64` static musl; macOS Intel and Apple Silicon; Windows `x64` and `ARM64`) from the latest [GitHub Release](https://github.com/opengeos/richdem-rs/releases), checks the release's `.sha256` checksum to catch corrupted downloads, and installs it to `$XDG_BIN_HOME` or `~/.local/bin` (`%USERPROFILE%\.local\bin` on Windows). No Rust or Python installation is required. (Checksums come from the same GitHub Release as the archives, so they guard against download corruption rather than acting as an independent release signature.)

```sh
# Linux and macOS
curl -LsSf https://richdem-rs.opengeos.org/install.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy ByPass -c "irm https://richdem-rs.opengeos.org/install.ps1 | iex"
```

Set `RICHDEM_VERSION` to install a specific release, or `RICHDEM_INSTALL_DIR` to choose another directory:

```sh
curl -LsSf https://richdem-rs.opengeos.org/install.sh | RICHDEM_VERSION=0.2.0 sh
```

```powershell
$env:RICHDEM_VERSION = "0.2.0"; irm https://richdem-rs.opengeos.org/install.ps1 | iex
```

On Windows, the installer adds the install directory to your user `PATH` (set `RICHDEM_NO_MODIFY_PATH=1` to skip). On Linux/macOS, it prints the `export PATH=...` line if needed. Re-run the installer anytime to upgrade.

### From crates.io

```sh
cargo install richdem-cli
```

### Prebuilt archives

You can also download release archives and `.sha256` checksums directly from [GitHub Releases](https://github.com/opengeos/richdem-rs/releases).

Verify the installation:

```sh
richdem --version
richdem --help
```

### Uninstall

Remove the installed binary (`~/.local/bin/richdem` on Linux/macOS, or `%USERPROFILE%\.local\bin\richdem.exe` on Windows), or run `cargo uninstall richdem-cli` if installed via Cargo.
