// Interactive browser playground for richdem-rs compiled to WebAssembly (richdem-js).
// Runs entirely client-side on Uint8Array DEM buffers — zero server uploads.
(function () {
  "use strict";

  const SCRIPT_SRC = document.currentScript ? document.currentScript.src : window.location.href;
  const SITE_ROOT = new URL("../", SCRIPT_SRC);
  const WASM_URL = new URL("wasm/richdem_js.js", SITE_ROOT).href;

  let modulePromise = null;
  let loadedFile = null; // { bytes: Uint8Array, name: string }
  let activeObjectUrls = [];
  let loadGeneration = 0;
  let runToken = 0;

  const $ = (id) => document.getElementById(id);

  function loadWasm() {
    if (!modulePromise) {
      modulePromise = import(WASM_URL).then(async (m) => {
        await m.default();
        return m;
      });
    }
    return modulePromise;
  }

  function setStatus(text, isError) {
    const el = $("rp-status");
    if (!el) return;
    el.textContent = text;
    el.classList.toggle("rp-error", !!isError);
  }

  function errMsg(e) {
    return e && e.message ? e.message : String(e);
  }

  function clearObjectUrls() {
    for (const url of activeObjectUrls) {
      URL.revokeObjectURL(url);
    }
    activeObjectUrls = [];
  }

  function nextTick() {
    return new Promise((resolve) => setTimeout(resolve, 0));
  }

  function renderCanvases(panels) {
    const container = $("rp-canvases");
    container.innerHTML = "";
    for (const { title, width, height, rgba, baseRgba, alpha } of panels) {
      const fig = document.createElement("figure");
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const ctx = canvas.getContext("2d");
      const out = new Uint8ClampedArray(width * height * 4);
      if (baseRgba) {
        const aVal = alpha ?? 0.78;
        const inv = 1 - aVal;
        for (let i = 0; i < out.length; i += 4) {
          if (rgba[i + 3] > 0) {
            out[i] = Math.round(baseRgba[i] * inv + rgba[i] * aVal);
            out[i + 1] = Math.round(baseRgba[i + 1] * inv + rgba[i + 1] * aVal);
            out[i + 2] = Math.round(baseRgba[i + 2] * inv + rgba[i + 2] * aVal);
            out[i + 3] = 255;
          } else {
            out[i] = baseRgba[i];
            out[i + 1] = baseRgba[i + 1];
            out[i + 2] = baseRgba[i + 2];
            out[i + 3] = 255;
          }
        }
      } else {
        out.set(rgba);
      }
      ctx.putImageData(new ImageData(out, width, height), 0, 0);
      const cap = document.createElement("figcaption");
      cap.textContent = title;
      fig.append(canvas, cap);
      container.appendChild(fig);
    }
  }

  function renderSummary(items) {
    const el = $("rp-summary");
    el.innerHTML = "";
    for (const [label, value] of items) {
      const card = document.createElement("div");
      card.className = "rp-stat";
      const valEl = document.createElement("strong");
      valEl.textContent = value;
      const lblEl = document.createElement("span");
      lblEl.textContent = label;
      card.append(valEl, lblEl);
      el.appendChild(card);
    }
  }

  function renderDownloads(files) {
    const el = $("rp-downloads");
    el.innerHTML = "";
    for (const { label, filename, data, mime } of files) {
      const url = URL.createObjectURL(new Blob([data], { type: mime }));
      activeObjectUrls.push(url);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.className = "md-button";
      a.textContent = label;
      el.appendChild(a);
    }
  }

  function renderDepressionTable(deps) {
    const sec = $("rp-table-section");
    if (!deps || !deps.length) {
      sec.hidden = true;
      return;
    }
    sec.hidden = false;
    const tbody = $("rp-tbody");
    tbody.innerHTML = "";
    const limit = 100;
    const shown = deps.slice(0, limit);
    $("rp-table-note").textContent =
      deps.length > limit
        ? `(showing largest ${limit} of ${deps.length} depressions)`
        : `(${deps.length} depressions)`;

    const fmt = (n) => (typeof n === "number" && Number.isFinite(n) ? n.toFixed(2) : "—");
    for (const d of shown) {
      const tr = document.createElement("tr");
      const cols = [
        d.id,
        d.parent === null ? "Ocean" : d.parent,
        d.cell_count,
        fmt(d.pit_elev),
        fmt(d.out_elev),
        fmt(d.depth),
        fmt(d.dep_vol),
        fmt(d.water_vol),
      ];
      for (const val of cols) {
        const td = document.createElement("td");
        td.textContent = String(val);
        tr.appendChild(td);
      }
      tbody.appendChild(tr);
    }
  }

  function syncWorkflowFields() {
    const wf = $("rp-workflow").value;
    document.querySelectorAll("#richdem-playground .rp-dep-only").forEach((el) => {
      el.hidden = wf !== "depressions";
    });
    document.querySelectorAll("#richdem-playground .rp-flow-only").forEach((el) => {
      el.hidden = wf !== "flow";
    });
    document.querySelectorAll("#richdem-playground .rp-terrain-only").forEach((el) => {
      el.hidden = wf !== "terrain";
    });
    document.querySelectorAll("#richdem-playground .rp-fsm-only").forEach((el) => {
      el.hidden = wf !== "overview" && wf !== "fsm";
    });
    document.querySelectorAll("#richdem-playground .rp-topo-only").forEach((el) => {
      el.hidden = wf !== "overview" && wf !== "fsm" && wf !== "depressions";
    });
  }

  async function runAnalysis() {
    if (!loadedFile) return;
    const token = ++runToken;
    const wf = $("rp-workflow").value;
    const topo = $("rp-topo").value;
    const waterDepth = Math.max(0.0, parseFloat($("rp-water-depth").value) || 1.5);
    const depMethod = $("rp-dep-method").value;
    const flowMethod = $("rp-flow-method").value;
    const exponent = parseFloat($("rp-exponent").value) || 1.1;
    const attrib = $("rp-attrib").value;

    const runBtn = $("rp-run");
    runBtn.disabled = true;
    setStatus(`Running richdem-rs (${wf}) on ${loadedFile.name} in WebAssembly…`);
    await nextTick();

    try {
      const wasm = await loadWasm();
      if (token !== runToken) return;

      const t0 = performance.now();
      const stem = loadedFile.name.replace(/\.[^.]+$/, "");
      const info = JSON.parse(wasm.rasterInfo(loadedFile.bytes, loadedFile.name));
      const w = info.width;
      const h = info.height;

      const panels = [];
      const downloads = [];
      let summary = [];
      let depressions = null;

      if (wf === "overview") {
        const filled = wasm.fillDepressions(loadedFile.bytes, loadedFile.name, true, topo);
        let accum = null;
        let slope = null;
        let fsm = null;
        try {
          const filledBytes = filled.tifBytes();
          accum = wasm.flowAccumulation(filledBytes, "filled.tif", "Dinf", 0, false, true);
          slope = wasm.terrainAttribute(filledBytes, "filled.tif", "slope_degrees", 1.0);
          fsm = wasm.depressionHierarchyFsm(loadedFile.bytes, loadedFile.name, waterDepth, topo);

          const hill = fsm.hillshadeRgba();
          panels.push(
            { title: "Shaded Relief (Input DEM)", width: w, height: h, rgba: hill },
            { title: `Fill-Spill-Merge Water (${waterDepth.toFixed(2)} m rain)`, width: w, height: h, rgba: fsm.waterDepthRgba(), baseRgba: hill, alpha: 0.8 },
            { title: "D-infinity Flow Accumulation (log)", width: w, height: h, rgba: accum.rgba() },
            { title: "Slope (degrees, Horn 1981)", width: w, height: h, rgba: slope.rgba() },
          );
          downloads.push(
            { label: "Filled DEM GeoTIFF", filename: `${stem}_filled.tif`, data: filledBytes, mime: "image/tiff" },
            { label: "D-inf Accumulation GeoTIFF", filename: `${stem}_dinf_accum.tif`, data: accum.tifBytes(), mime: "image/tiff" },
            { label: "Slope (deg) GeoTIFF", filename: `${stem}_slope_deg.tif`, data: slope.tifBytes(), mime: "image/tiff" },
            { label: "FSM Water Depth GeoTIFF", filename: `${stem}_fsm_water.tif`, data: fsm.waterDepthTif(), mime: "image/tiff" },
            { label: "Depression Labels GeoTIFF", filename: `${stem}_dep_labels.tif`, data: fsm.labelsTif(), mime: "image/tiff" },
          );
          depressions = JSON.parse(fsm.depressionsJson());
          const elapsed = performance.now() - t0;
          summary = [
            ["Raster Size", `${w} × ${h}`],
            ["Elevation", `${info.min.toFixed(1)} – ${info.max.toFixed(1)}`],
            ["Filled Cells", filled.modifiedCells.toLocaleString()],
            ["Max Dinf Accum", accum.max.toFixed(0)],
            ["Max Slope", `${slope.max.toFixed(1)}°`],
            ["Depressions", String(fsm.numDepressions)],
            ["WASM Runtime", `${elapsed.toFixed(0)} ms`],
          ];
        } finally {
          if (fsm) fsm.free();
          if (slope) slope.free();
          if (accum) accum.free();
          filled.free();
        }
      } else if (wf === "fsm") {
        const fsm = wasm.depressionHierarchyFsm(loadedFile.bytes, loadedFile.name, waterDepth, topo);
        try {
          const hill = fsm.hillshadeRgba();
          panels.push(
            { title: `Fill-Spill-Merge Water (${waterDepth.toFixed(2)} m rain)`, width: w, height: h, rgba: fsm.waterDepthRgba(), baseRgba: hill, alpha: 0.82 },
            { title: "Depression Hierarchy Basins (over hillshade)", width: w, height: h, rgba: fsm.labelsRgba(), baseRgba: hill, alpha: 0.65 },
          );
          downloads.push(
            { label: "FSM Water Depth GeoTIFF", filename: `${stem}_fsm_water.tif`, data: fsm.waterDepthTif(), mime: "image/tiff" },
            { label: "Depression Labels GeoTIFF", filename: `${stem}_dep_labels.tif`, data: fsm.labelsTif(), mime: "image/tiff" },
          );
          depressions = JSON.parse(fsm.depressionsJson());
          const elapsed = performance.now() - t0;
          summary = [
            ["Raster Size", `${w} × ${h}`],
            ["Depressions", String(fsm.numDepressions)],
            ["Inundated Cells", fsm.inundatedCells.toLocaleString()],
            ["Max Water Depth", `${fsm.maxWaterDepth.toFixed(2)} m`],
            ["Water Volume", fsm.totalWaterVolume.toLocaleString(undefined, { maximumFractionDigits: 0 })],
            ["WASM Runtime", `${elapsed.toFixed(0)} ms`],
          ];
        } finally {
          fsm.free();
        }
      } else if (wf === "depressions") {
        const hill = wasm.hillshadeRgba(loadedFile.bytes, loadedFile.name, 315.0, 45.0, 1.0);
        let out = null;
        try {
          if (depMethod === "fill_eps") {
            out = wasm.fillDepressions(loadedFile.bytes, loadedFile.name, true, topo);
          } else if (depMethod === "fill") {
            out = wasm.fillDepressions(loadedFile.bytes, loadedFile.name, false, topo);
          } else {
            const mode = depMethod.replace("breach_", "");
            out = wasm.breachDepressions(loadedFile.bytes, loadedFile.name, mode, 0, 0, true, false);
          }
          panels.push(
            { title: "Conditioned DEM (Shaded Relief)", width: w, height: h, rgba: out.rgba() },
            { title: "Modified Elevation Diff (over hillshade)", width: w, height: h, rgba: out.diffRgba(), baseRgba: hill, alpha: 0.85 },
          );
          downloads.push({
            label: `${depMethod} GeoTIFF`,
            filename: `${stem}_${depMethod}.tif`,
            data: out.tifBytes(),
            mime: "image/tiff",
          });
          const elapsed = performance.now() - t0;
          summary = [
            ["Raster Size", `${w} × ${h}`],
            ["Modified Cells", out.modifiedCells.toLocaleString()],
            ["Min Elevation", out.min.toFixed(2)],
            ["Max Elevation", out.max.toFixed(2)],
            ["WASM Runtime", `${elapsed.toFixed(0)} ms`],
          ];
        } finally {
          if (out) out.free();
        }
      } else if (wf === "flow") {
        const hill = wasm.hillshadeRgba(loadedFile.bytes, loadedFile.name, 315.0, 45.0, 1.0);
        const accum = wasm.flowAccumulation(loadedFile.bytes, loadedFile.name, flowMethod, exponent, true, true);
        try {
          panels.push(
            { title: "Shaded Relief (Input DEM)", width: w, height: h, rgba: hill },
            { title: `${flowMethod} Flow Accumulation (log-scaled)`, width: w, height: h, rgba: accum.rgba() },
          );
          downloads.push({
            label: `${flowMethod} Flow Accum GeoTIFF`,
            filename: `${stem}_${flowMethod.toLowerCase()}_accum.tif`,
            data: accum.tifBytes(),
            mime: "image/tiff",
          });
          const elapsed = performance.now() - t0;
          summary = [
            ["Raster Size", `${w} × ${h}`],
            ["Method", flowMethod],
            ["Min Accum", accum.min.toFixed(2)],
            ["Max Accum", accum.max.toFixed(1)],
            ["WASM Runtime", `${elapsed.toFixed(0)} ms`],
          ];
        } finally {
          accum.free();
        }
      } else if (wf === "terrain") {
        const hill = wasm.hillshadeRgba(loadedFile.bytes, loadedFile.name, 315.0, 45.0, 1.0);
        const ta = wasm.terrainAttribute(loadedFile.bytes, loadedFile.name, attrib, 1.0);
        try {
          panels.push(
            { title: "Shaded Relief (Input DEM)", width: w, height: h, rgba: hill },
            { title: attrib, width: w, height: h, rgba: ta.rgba() },
          );
          downloads.push({
            label: `${attrib} GeoTIFF`,
            filename: `${stem}_${attrib}.tif`,
            data: ta.tifBytes(),
            mime: "image/tiff",
          });
          const elapsed = performance.now() - t0;
          summary = [
            ["Raster Size", `${w} × ${h}`],
            ["Attribute", attrib],
            ["Min", ta.min.toFixed(3)],
            ["Max", ta.max.toFixed(3)],
            ["WASM Runtime", `${elapsed.toFixed(0)} ms`],
          ];
        } finally {
          ta.free();
        }
      }

      if (token !== runToken) return;
      clearObjectUrls();
      $("rp-results").hidden = false;
      const sliderBar = $("rp-fsm-slider-bar");
      sliderBar.hidden = wf !== "fsm";
      if (wf === "fsm") {
        $("rp-water-slider").value = String(waterDepth);
        $("rp-water-label").textContent = `${waterDepth.toFixed(2)} m`;
      }
      renderSummary(summary);
      renderCanvases(panels);
      renderDownloads(downloads);
      renderDepressionTable(depressions);
      setStatus(
        `richdem-rs ${wasm.version()}: completed ${wf} on ${loadedFile.name} (${w}×${h}).`,
      );
    } catch (e) {
      if (token === runToken) {
        setStatus(`Analysis failed: ${errMsg(e)}`, true);
      }
    } finally {
      if ($("rp-run")) {
        $("rp-run").disabled = !loadedFile;
      }
    }
  }

  async function loadFile(file) {
    const gen = ++loadGeneration;
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (gen !== loadGeneration) return;
      loadedFile = { bytes, name: file.name };
      $("rp-run").disabled = false;
      await runAnalysis();
    } catch (e) {
      if (gen === loadGeneration) {
        setStatus(`Could not read ${file.name}: ${errMsg(e)}`, true);
      }
    }
  }

  async function loadPerlin(size, seed, relief, cellSize) {
    const gen = ++loadGeneration;
    try {
      setStatus(`Generating ${size}×${size} Perlin terrain (seed ${seed}) in WebAssembly…`);
      await nextTick();
      const wasm = await loadWasm();
      if (gen !== loadGeneration) return;
      const bytes = wasm.generatePerlinDem(size, seed, relief, cellSize);
      if (gen !== loadGeneration) return;
      loadedFile = { bytes, name: `perlin_seed_${seed}.tif` };
      $("rp-run").disabled = false;
      await runAnalysis();
    } catch (e) {
      if (gen === loadGeneration) {
        setStatus(`Could not generate terrain: ${errMsg(e)}`, true);
      }
    }
  }

  function init() {
    const root = $("richdem-playground");
    if (!root) {
      loadGeneration++;
      runToken++;
      clearObjectUrls();
      loadedFile = null;
      return;
    }
    if (root.dataset.ready) return;
    root.dataset.ready = "1";

    syncWorkflowFields();
    loadWasm().then(
      (m) =>
        setStatus(
          `richdem-rs ${m.version()} WebAssembly is ready. Drop a DEM or generate a Perlin terrain below.`,
        ),
      (e) => setStatus(`Could not load richdem-rs WebAssembly module: ${errMsg(e)}`, true),
    );

    $("rp-workflow").addEventListener("change", () => {
      syncWorkflowFields();
      if (loadedFile) runAnalysis();
    });
    $("rp-run").addEventListener("click", runAnalysis);
    $("rp-water-slider").addEventListener("input", (e) => {
      const v = parseFloat(e.target.value);
      $("rp-water-depth").value = String(v);
      $("rp-water-label").textContent = `${v.toFixed(2)} m`;
    });
    $("rp-water-slider").addEventListener("change", () => {
      if (loadedFile) runAnalysis();
    });

    root.querySelectorAll("[data-perlin]").forEach((btn) => {
      btn.addEventListener("click", () => {
        const [size, seed, relief, cell] = btn.dataset.perlin.split(",").map(Number);
        loadPerlin(size, seed, relief, cell);
      });
    });

    $("rp-file").addEventListener("change", (e) => {
      if (e.target.files[0]) loadFile(e.target.files[0]);
    });

    const drop = $("rp-drop");
    drop.addEventListener("dragover", (e) => {
      e.preventDefault();
      drop.classList.add("rp-over");
    });
    drop.addEventListener("dragleave", () => drop.classList.remove("rp-over"));
    drop.addEventListener("drop", (e) => {
      e.preventDefault();
      drop.classList.remove("rp-over");
      if (e.dataTransfer.files[0]) loadFile(e.dataTransfer.files[0]);
    });
  }

  if (window.document$) {
    window.document$.subscribe(init);
  } else {
    document.addEventListener("DOMContentLoaded", init);
  }
})();
