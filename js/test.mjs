import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import * as richdem from "./pkg-node/richdem_js.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootCargoToml = fs.readFileSync(path.resolve(__dirname, "..", "Cargo.toml"), "utf8");
const jsCargoToml = fs.readFileSync(path.resolve(__dirname, "Cargo.toml"), "utf8");
const wsVersion = rootCargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const jsVersion = jsCargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
assert.ok(wsVersion, "expected workspace version in root Cargo.toml");
assert.equal(jsVersion, wsVersion, "js/Cargo.toml version must match workspace version");

console.log(`richdem-js version: ${richdem.version()}`);
assert.equal(richdem.version(), wsVersion);

// 1. Generate synthetic Perlin DEM in WASM + inspect metadata
const demBytes = richdem.generatePerlinDem(128, 42, 120.0, 10.0);
assert.ok(demBytes.length > 1000);
const info = JSON.parse(richdem.rasterInfo(demBytes, "perlin.tif"));
assert.equal(info.width, 128);
assert.equal(info.height, 128);
assert.ok(info.max > info.min);
assert.equal(info.has_depressions, true);

// 2. Hillshade preview
const hs = richdem.hillshadeRgba(demBytes, "perlin.tif", 315, 45, 1.0);
assert.equal(hs.length, 128 * 128 * 4);

// 3. Priority-Flood + Epsilon depression filling
const filled = richdem.fillDepressions(demBytes, "perlin.tif", true, "D8");
assert.equal(filled.width, 128);
assert.equal(filled.height, 128);
assert.ok(filled.modifiedCells > 0);
const filledBytes = filled.tifBytes();
const filledInfo = JSON.parse(richdem.rasterInfo(filledBytes, "filled.tif"));
assert.equal(filledInfo.has_depressions, false);
filled.free();

// 4. Lindsay (2016) breaching (complete, selective, constrained; 0 = unlimited limits)
for (const mode of ["complete", "selective", "constrained"]) {
  const breached = richdem.breachDepressions(demBytes, "perlin.tif", mode, 0, 0, true, false);
  assert.ok(breached.modifiedCells > 0, `expected modifiedCells > 0 for breach mode ${mode}`);
  breached.free();
}

// 5. Flat resolution
const resolved = richdem.resolveFlats(filledBytes, "filled.tif", "epsilon");
assert.equal(resolved.width, 128);
resolved.free();

// 6. Flow accumulation (Dinf, D8, Quinn)
for (const method of ["Dinf", "D8", "Quinn"]) {
  const accum = richdem.flowAccumulation(filledBytes, "filled.tif", method, 0, false, true);
  assert.ok(accum.max > 1.0, `expected accumulation > 1 for ${method}, got ${accum.max}`);
  assert.equal(accum.rgba().length, 128 * 128 * 4);
  accum.free();
}

// 7. Terrain attributes (slope, aspect, curvature, SPI, CTI)
for (const attr of ["slope_degrees", "aspect", "curvature", "spi", "cti"]) {
  const ta = richdem.terrainAttribute(filledBytes, "filled.tif", attr, 1.0);
  assert.equal(ta.rgba().length, 128 * 128 * 4);
  assert.ok(ta.tifBytes().length > 100);
  ta.free();
}

// 8. Depression Hierarchy + Fill-Spill-Merge (Barnes 2020/2021)
const fsm = richdem.depressionHierarchyFsm(demBytes, "perlin.tif", 1.5, "D8");
assert.ok(fsm.numDepressions >= 1);
assert.ok(fsm.inundatedCells > 0);
assert.ok(fsm.maxWaterDepth > 0);
assert.ok(fsm.totalWaterVolume > 0);
assert.equal(fsm.waterDepthRgba().length, 128 * 128 * 4);
assert.equal(fsm.labelsRgba().length, 128 * 128 * 4);
const deps = JSON.parse(fsm.depressionsJson());
assert.equal(deps.length, fsm.numDepressions);
console.log(
  `WASM FSM OK: ${fsm.numDepressions} depressions, ${fsm.inundatedCells} inundated cells, ` +
    `max depth=${fsm.maxWaterDepth.toFixed(2)}m`,
);
fsm.free();

console.log("ALL richdem-js WASM TESTS PASSED!");
