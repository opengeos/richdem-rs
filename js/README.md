# `richdem-js` (WebAssembly bindings for `richdem-rs`)

Run Priority-Flood depression filling, Lindsay (2016) depression breaching, flat resolution, D8/D-infinity/MFD flow accumulation, Horn/Zevenbergen-Thorne terrain attributes, and Barnes (2020/2021) Depression Hierarchy + Fill-Spill-Merge directly in **browsers and Node.js** via WebAssembly — zero GDAL, zero C/C++, zero server required.

## Building

```sh
# For browsers (ES module)
wasm-pack build --target web --release --out-dir pkg -- --locked

# For Node.js + test suite
wasm-pack build --target nodejs --release --out-dir pkg-node -- --locked
node test.mjs
```

## Browser Usage

```js
import init, {
  fillDepressions,
  flowAccumulation,
  terrainAttribute,
  depressionHierarchyFsm,
} from "./pkg/richdem_js.js";

await init();
const demBytes = new Uint8Array(await (await fetch("dem.tif")).arrayBuffer());

const filled = fillDepressions(demBytes, "dem.tif", true, "D8");
try {
  const accum = flowAccumulation(filled.tifBytes(), "filled.tif", "Dinf", 0, false, true);
  try {
    console.log("Max D-inf accumulation:", accum.max);
  } finally {
    accum.free();
  }
} finally {
  filled.free();
}

const fsm = depressionHierarchyFsm(demBytes, "dem.tif", 1.5, "D8");
try {
  console.log("Depressions:", fsm.numDepressions, "Inundated cells:", fsm.inundatedCells);
} finally {
  fsm.free();
}
```

## License

`richdem-js` is licensed under the GNU General Public License v3.0 (`GPL-3.0-only`). See [LICENSE](LICENSE) and [NOTICE](NOTICE).
