# Playground

Run **`richdem-rs`** directly in your browser via WebAssembly — Priority-Flood depression filling, Lindsay (2016) breaching, flat resolution, D8/D-infinity/MFD flow accumulation, Horn/Zevenbergen-Thorne terrain attributes, and Barnes (2020/2021) Depression Hierarchy + Fill-Spill-Merge all execute locally in browser memory. Files are **never uploaded** to any server.

<div id="richdem-playground" markdown="0">
  <p class="rp-status" id="rp-status">Loading richdem-rs WebAssembly module…</p>

  <div class="rp-drop" id="rp-drop">
    <p><strong>Drop a DEM GeoTIFF or ASCII grid here</strong> or <label class="rp-link">choose a file<input type="file" id="rp-file" accept=".tif,.tiff,.asc,.dem,.img" class="rp-visually-hidden"></label>.</p>
    <p class="rp-hint">Single-band GeoTIFF / COG / ESRI ASCII Grid elevation rasters. Everything stays in your browser memory.</p>
    <p class="rp-samples">Or generate a synthetic Perlin DEM instantly in WASM:
      <button type="button" data-perlin="300,42,140,10">Mountain Terrain (300×300, seed 42)</button>
      <button type="button" data-perlin="300,137,90,10">Rolling Basin (300×300, seed 137)</button>
      <button type="button" data-perlin="300,7,180,10">Alpine Ridges (300×300, seed 7)</button>
    </p>
  </div>

  <div class="rp-controls" id="rp-controls">
    <div class="rp-field">
      <label for="rp-workflow">Workflow</label>
      <select id="rp-workflow">
        <option value="overview">Full Hydrologic Suite (Fill + Dinf + Slope + FSM)</option>
        <option value="fsm">Depression Hierarchy &amp; Fill-Spill-Merge</option>
        <option value="depressions">Depression Filling &amp; Breaching</option>
        <option value="flow">Flow Accumulation</option>
        <option value="terrain">Terrain Attributes</option>
      </select>
    </div>

    <div class="rp-field rp-dep-only" hidden>
      <label for="rp-dep-method">Depression method</label>
      <select id="rp-dep-method">
        <option value="fill_eps">Priority-Flood + Epsilon (Barnes 2014)</option>
        <option value="fill">Priority-Flood (Barnes 2014)</option>
        <option value="breach_complete">Lindsay Complete Breaching (2016)</option>
        <option value="breach_selective">Lindsay Selective Breaching (2016)</option>
        <option value="breach_constrained">Lindsay Constrained Breaching (2016)</option>
      </select>
    </div>

    <div class="rp-field rp-flow-only" hidden>
      <label for="rp-flow-method">Flow method</label>
      <select id="rp-flow-method">
        <option value="Dinf">D-infinity (Tarboton 1997)</option>
        <option value="D8">D8 (O'Callaghan &amp; Mark 1984)</option>
        <option value="D4">D4</option>
        <option value="Quinn">Quinn (1991)</option>
        <option value="Holmgren">Holmgren (1994)</option>
        <option value="Freeman">Freeman (1991)</option>
        <option value="Rho8">Rho8 (Fairfield &amp; Leymarie 1991)</option>
        <option value="Rho4">Rho4</option>
      </select>
    </div>

    <div class="rp-field rp-flow-only" hidden>
      <label for="rp-exponent">MFD exponent</label>
      <input type="number" id="rp-exponent" value="1.1" min="0.1" max="10" step="0.1">
    </div>

    <div class="rp-field rp-terrain-only" hidden>
      <label for="rp-attrib">Terrain attribute</label>
      <select id="rp-attrib">
        <option value="slope_degrees">Slope (degrees)</option>
        <option value="slope_percentage">Slope (%)</option>
        <option value="slope_riserun">Slope (rise/run)</option>
        <option value="aspect">Aspect (degrees)</option>
        <option value="curvature">Total Curvature</option>
        <option value="planform_curvature">Planform Curvature</option>
        <option value="profile_curvature">Profile Curvature</option>
        <option value="spi">Stream Power Index (SPI)</option>
        <option value="cti">Compound Topographic Index (CTI / TWI)</option>
      </select>
    </div>

    <div class="rp-field rp-fsm-only">
      <label for="rp-water-depth">Standing rainfall depth (m)</label>
      <input type="number" id="rp-water-depth" value="1.5" min="0.05" max="50" step="0.25">
    </div>

    <div class="rp-field rp-topo-only">
      <label for="rp-topo">Topology</label>
      <select id="rp-topo">
        <option value="D8">D8 (8-neighbor)</option>
        <option value="D4">D4 (4-neighbor)</option>
      </select>
    </div>

    <div class="rp-field rp-action">
      <button type="button" id="rp-run" class="md-button md-button--primary" disabled>Run analysis</button>
    </div>
  </div>

  <div id="rp-results" hidden>
    <div class="rp-summary" id="rp-summary"></div>

    <div id="rp-fsm-slider-bar" class="rp-slider-bar" hidden>
      <label for="rp-water-slider"><strong>Interactive Fill-Spill-Merge rainfall depth:</strong> <span id="rp-water-label">1.50 m</span></label>
      <input type="range" id="rp-water-slider" min="0.05" max="8.0" step="0.05" value="1.5">
    </div>

    <h3>Visualizations</h3>
    <div class="rp-canvases" id="rp-canvases"></div>

    <h3>Downloads</h3>
    <div class="rp-downloads" id="rp-downloads"></div>

    <div id="rp-table-section" hidden>
      <h3>Depression Hierarchy <span class="rp-hint" id="rp-table-note"></span></h3>
      <div class="rp-table-wrap">
        <table id="rp-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Parent</th>
              <th>Cells</th>
              <th>Pit Elev</th>
              <th>Outlet Elev</th>
              <th>Depth</th>
              <th>Capacity Vol</th>
              <th>Water Vol</th>
            </tr>
          </thead>
          <tbody id="rp-tbody"></tbody>
        </table>
      </div>
    </div>
  </div>
</div>

See [JavaScript / WebAssembly](javascript.md) for the full API reference and Node.js / browser usage examples.
