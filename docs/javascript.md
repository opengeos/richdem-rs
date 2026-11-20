# JavaScript / WebAssembly

`richdem-js` compiles [`richdem-rs`](https://github.com/opengeos/richdem-rs) to WebAssembly via `wasm-bindgen` for browsers and Node.js. All hydrologic and terrain algorithms run completely in memory on `Uint8Array` GeoTIFF / AAIGrid buffers with zero filesystem, GDAL, or C/C++ dependencies.

Try it right now with no installation on the [Playground](playground.md).

## Quick Start (Browser)

```js
import init, {
  version,
  rasterInfo,
  fillDepressions,
  breachDepressions,
  flowAccumulation,
  terrainAttribute,
  depressionHierarchyFsm,
} from "./wasm/richdem_js.js";

await init();
console.log("richdem-rs", version());

const demBytes = new Uint8Array(await (await fetch("dem.tif")).arrayBuffer());
const info = JSON.parse(rasterInfo(demBytes, "dem.tif"));
console.log(`${info.width}×${info.height}, depressions: ${info.has_depressions}`);

// 1. Fill depressions with epsilon downhill gradient
const filled = fillDepressions(demBytes, "dem.tif", true, "D8");
try {
  // 2. Compute D-infinity flow accumulation
  const accum = flowAccumulation(filled.tifBytes(), "filled.tif", "Dinf", 0, false, true);
  try {
    const canvas = document.querySelector("#accum");
    canvas.width = accum.width;
    canvas.height = accum.height;
    canvas.getContext("2d").putImageData(
      new ImageData(new Uint8ClampedArray(accum.rgba()), accum.width, accum.height),
      0,
      0,
    );
  } finally {
    accum.free();
  }
} finally {
  filled.free();
}

// 3. Barnes (2020/2021) Depression Hierarchy + Fill-Spill-Merge water routing
const fsm = depressionHierarchyFsm(demBytes, "dem.tif", 1.5, "D8");
try {
  console.log(`Depressions: ${fsm.numDepressions}, inundated cells: ${fsm.inundatedCells}`);
  console.log("Top depressions:", JSON.parse(fsm.depressionsJson()).slice(0, 5));
} finally {
  fsm.free();
}
```

## Quick Start (Node.js)

```sh
cd js
wasm-pack build --target nodejs --release --out-dir pkg-node -- --locked
```

```js
import fs from "node:fs";
import * as richdem from "./pkg-node/richdem_js.js";

const demBytes = fs.readFileSync("dem.tif");
const filled = richdem.fillDepressions(demBytes, "dem.tif", true, "D8");
try {
  fs.writeFileSync("filled.tif", filled.tifBytes());
  const slope = richdem.terrainAttribute(filled.tifBytes(), "filled.tif", "slope_degrees", 1.0);
  try {
    fs.writeFileSync("slope_deg.tif", slope.tifBytes());
  } finally {
    slope.free();
  }
} finally {
  filled.free();
}
```

## API Reference

| Function | Signature | Returns |
|---|---|---|
| `version` | `()` | `string` — `richdem-rs` crate version |
| `rasterInfo` | `(bytes, filename)` | JSON string (`width`, `height`, `geotransform`, `projection`, `no_data`, `min`, `max`, `has_depressions`) |
| `generatePerlinDem` | `(size, seed, reliefM, cellSizeM)` | `Uint8Array` — synthetic Perlin-noise GeoTIFF DEM |
| `hillshadeRgba` | `(bytes, filename, azimuthDeg, elevationDeg, zFactor)` | `Uint8Array` — `width × height × 4` shaded hypsometric relief RGBA pixels |
| `fillDepressions` | `(bytes, filename, epsilon, topology)` | `RasterOutput` (`width`, `height`, `min`, `max`, `modifiedCells`, `projection`, `tifBytes()`, `rgba()`, `diffRgba()`) |
| `breachDepressions` | `(bytes, filename, mode, maxPathLen, maxDepth, epsGradients, fillRemaining)` | `RasterOutput` (`mode`: `"complete"`, `"selective"`, or `"constrained"`) |
| `resolveFlats` | `(bytes, filename, method)` | `RasterOutput` (`method`: `"epsilon"`, `"barnes"`, or `"garbrecht"`) |
| `flowAccumulation` | `(bytes, filename, method, exponent, prefill, logPreview)` | `RasterOutput` (`method`: `Dinf`, `D8`, `D4`, `Quinn`, `Holmgren`, `Freeman`, `Rho8`, `Rho4`) |
| `terrainAttribute` | `(bytes, filename, attrib, zScale)` | `RasterOutput` (`slope_degrees`, `slope_percentage`, `slope_riserun`, `slope_radians`, `aspect`, `curvature`, `planform_curvature`, `profile_curvature`, `spi`, `cti`) |
| `depressionHierarchyFsm` | `(bytes, filename, waterDepth, topology)` | `FsmOutput` (`width`, `height`, `projection`, `numDepressions`, `inundatedCells`, `maxWaterDepth`, `totalWaterVolume`, `depressionsJson()`, `waterDepthTif()`, `labelsTif()`, `hillshadeRgba()`, `waterDepthRgba()`, `labelsRgba()`) |

!!! tip "Memory Management"
    `RasterOutput` and `FsmOutput` hold WebAssembly heap allocations. Call `.free()` (ideally in a `finally` block) once you have copied the buffers or strings you need.

## Building from Source

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

cd js
# Browser ES module
wasm-pack build --target web --release --out-dir pkg -- --locked

# Node.js module + end-to-end test suite
wasm-pack build --target nodejs --release --out-dir pkg-node -- --locked
node test.mjs
```

## License

`richdem-js` is released under the GNU General Public License v3.0 (`GPL-3.0-only`). See [LICENSE](https://github.com/opengeos/richdem-rs/blob/main/LICENSE) and [NOTICE](https://github.com/opengeos/richdem-rs/blob/main/NOTICE).
