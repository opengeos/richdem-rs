//! Command-line interface for richdem-rs, covering RichDEM's `rd_*` utilities
//! with pure-Rust raster I/O via `terrio`.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use richdem::{
    Array2D, DynamicArray2D, FlowAccumMethod, LindsayMode, NO_DEP, NO_FLOW, OCEAN, TerrainAttr,
    Topology, breach_depressions, convert_arc_flowdirs_to_richdem_d8,
    convert_taudem_d8_to_richdem_d8, d8_flow_accum, d8_flow_directions, dem_surface_area,
    fill_depressions, fill_depressions_epsilon, fill_spill_merge, find_flow_loops,
    flow_accumulation_by_method, format_depression_hierarchy, get_depression_hierarchy,
    has_depressions, lindsay2016, load_dynamic, load_raster, pit_mask,
    priority_flood_flowdirs_barnes2014, resolve_flats_epsilon, save_dynamic, save_raster,
    terrain_attribute,
};

#[derive(Parser)]
#[command(
    name = "richdem",
    version,
    about = "High-performance DEM hydrologic and terrain analysis (pure-Rust port of Richard Barnes's RichDEM)",
    long_about = "Pure-Rust port of RichDEM (https://github.com/r-barnes/richdem) using terrio for GDAL-free geospatial raster I/O."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum CliTopology {
    D8,
    D4,
}

impl From<CliTopology> for Topology {
    fn from(t: CliTopology) -> Self {
        match t {
            CliTopology::D8 => Topology::D8,
            CliTopology::D4 => Topology::D4,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliBreachMode {
    Complete,
    Selective,
    Constrained,
}

#[derive(Subcommand)]
enum Commands {
    /// Fill all depressions in a DEM (rd_depressions_flood / rd_depression_filling)
    DepressionsFlood {
        /// Input DEM raster
        input: PathBuf,
        /// Output raster
        output: PathBuf,
        /// Enforce an epsilon downhill gradient across filled areas (floating-point DEMs)
        #[arg(short = 'g', long)]
        gradient: bool,
        /// Connectivity topology (d8 or d4)
        #[arg(short = 't', long, value_enum, default_value_t = CliTopology::D8)]
        topology: CliTopology,
        /// Compress output GeoTIFF with DEFLATE
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Breach depressions in a DEM using Lindsay (2016) (rd_depressions_breach)
    DepressionsBreach {
        /// Input DEM raster
        input: PathBuf,
        /// Output raster
        output: PathBuf,
        /// Breaching mode
        #[arg(short = 'm', long, value_enum, default_value_t = CliBreachMode::Complete)]
        mode: CliBreachMode,
        /// Maximum breach path length (selective/constrained modes)
        #[arg(long, default_value_t = u32::MAX)]
        max_length: u32,
        /// Maximum breach depth (selective/constrained modes)
        #[arg(long, default_value_t = 1e30)]
        max_depth: f64,
        /// Fill remaining unbreached depressions (selective/constrained modes)
        #[arg(long)]
        fill_depressions: bool,
        /// Apply epsilon gradients along breached paths
        #[arg(short = 'g', long)]
        gradient: bool,
        /// Connectivity topology (d8 or d4; complete mode only)
        #[arg(short = 't', long, value_enum, default_value_t = CliTopology::D8)]
        topology: CliTopology,
        /// Compress output GeoTIFF with DEFLATE
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Check whether a DEM contains depressions (rd_depressions_has)
    DepressionsHas {
        /// Input DEM raster
        input: PathBuf,
        /// Connectivity topology
        #[arg(short = 't', long, value_enum, default_value_t = CliTopology::D8)]
        topology: CliTopology,
    },
    /// Generate a depression mask (1=pit, 0=non-pit, 3=NoData) (rd_depressions_mask)
    DepressionsMask {
        /// Input DEM raster
        input: PathBuf,
        /// Output mask raster
        output: PathBuf,
        /// Connectivity topology
        #[arg(short = 't', long, value_enum, default_value_t = CliTopology::D8)]
        topology: CliTopology,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Resolve flat surfaces by imposing minimal epsilon gradients (Barnes 2014)
    ResolveFlats {
        /// Input DEM raster
        input: PathBuf,
        /// Output raster
        output: PathBuf,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Compute D8 steepest-descent flow directions (rd_d8_flowdirs)
    D8Flowdirs {
        /// Input DEM raster
        input: PathBuf,
        /// Output D8 flow direction raster
        output: PathBuf,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Implicitly fill depressions and compute D8 flow directions (rd_flood_for_flowdirs)
    FloodForFlowdirs {
        /// Input DEM raster
        input: PathBuf,
        /// Output D8 flow direction raster
        output: PathBuf,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Compute flow accumulation using a named flow metric (rd_flow_accumulation)
    FlowAccumulation {
        /// Input DEM raster (or D8 flow direction raster with --from-d8)
        input: PathBuf,
        /// Output flow accumulation raster
        output: PathBuf,
        /// Flow accumulation method (D8, D4, Tarboton, Dinf, Quinn, Holmgren, Freeman, Rho8, Rho4)
        #[arg(short = 'm', long, default_value = "D8")]
        method: String,
        /// Exponent for Holmgren or Freeman methods
        #[arg(short = 'e', long)]
        exponent: Option<f64>,
        /// Treat input as precomputed RichDEM D8 flow directions
        #[arg(long)]
        from_d8: bool,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Compute terrain attributes: slope, aspect, curvature (rd_terrain_property)
    TerrainProperty {
        /// Input DEM raster
        input: PathBuf,
        /// Output raster
        output: PathBuf,
        /// Attribute name (slope_riserun, slope_percentage, slope_degrees, slope_radians, aspect, curvature, planform_curvature, profile_curvature)
        #[arg(short = 'a', long)]
        attrib: String,
        /// Z-scale elevation multiplier
        #[arg(short = 'z', long, default_value_t = 1.0)]
        zscale: f32,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Compute topographic surface area via Jenness (2004) (rd_surface_area)
    SurfaceArea {
        /// Input DEM raster
        input: PathBuf,
        /// Z-scale elevation multiplier
        #[arg(short = 'z', long, default_value_t = 1.0)]
        zscale: f64,
    },
    /// Build and display a Depression Hierarchy (rd_depression_hierarchy)
    DepressionHierarchy {
        /// Input DEM raster
        input: PathBuf,
        /// Optional output label raster
        #[arg(short = 'l', long)]
        labels_out: Option<PathBuf>,
        /// Optional output flowdirs raster
        #[arg(short = 'f', long)]
        flowdirs_out: Option<PathBuf>,
        /// Connectivity topology
        #[arg(short = 't', long, value_enum, default_value_t = CliTopology::D8)]
        topology: CliTopology,
    },
    /// Run Fill-Spill-Merge hydrological routing (rd_fill_spill_merge)
    FillSpillMerge {
        /// Input DEM raster
        input: PathBuf,
        /// Output water table depth raster
        output: PathBuf,
        /// Uniform initial surface water depth (if --wtd-in is omitted)
        #[arg(short = 'w', long, default_value_t = 1.0)]
        initial_water: f64,
        /// Optional input water table depth raster
        #[arg(long)]
        wtd_in: Option<PathBuf>,
        /// Compress output GeoTIFF
        #[arg(short = 'c', long)]
        compress: bool,
    },
    /// Print raster metadata and processing history (rd_info)
    Info {
        /// Input raster
        input: PathBuf,
    },
    /// Compare two rasters cell-by-cell and metadata-wise (rd_compare)
    Compare {
        /// First raster
        first: PathBuf,
        /// Second raster
        second: PathBuf,
    },
    /// Display or set a raster's 6-element geotransform (rd_geotransform)
    Geotransform {
        /// Input raster
        input: PathBuf,
        /// Output raster (required when setting geotransform unless --in-place is passed)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// Overwrite input raster in place when modifying metadata
        #[arg(long)]
        in_place: bool,
        /// Set 6 geotransform values
        #[arg(long, num_args = 6, allow_hyphen_values = true)]
        set: Option<Vec<f64>>,
    },
    /// Display or set a raster's NoData value (rd_no_data)
    NoData {
        /// Input raster
        input: PathBuf,
        /// Output raster (required when setting NoData unless --in-place is passed)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// Overwrite input raster in place when modifying metadata
        #[arg(long)]
        in_place: bool,
        /// New NoData value
        #[arg(long, allow_hyphen_values = true)]
        set: Option<String>,
    },
    /// Display or set a raster's projection CRS (rd_projection)
    Projection {
        /// Input raster
        input: PathBuf,
        /// Output raster (required when setting projection unless --in-place is passed)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// Overwrite input raster in place when modifying metadata
        #[arg(long)]
        in_place: bool,
        /// Projection string (EPSG:xxxx, WKT, or PROJ)
        #[arg(long)]
        set: Option<String>,
        /// Copy projection from another raster
        #[arg(long)]
        from: Option<PathBuf>,
    },
    /// Display a raster's processing history (rd_processing_history)
    ProcessingHistory {
        /// Input raster
        input: PathBuf,
    },
    /// Print value histogram of a raster (rd_hist)
    Hist {
        /// Input raster
        input: PathBuf,
    },
    /// Check a D8 flow direction raster for cycles/loops (rd_loop_check)
    LoopCheck {
        /// Input D8 flow direction raster
        input: PathBuf,
    },
    /// Expand raster dimensions padding with NoData (rd_expand_dimensions)
    ExpandDimensions {
        /// Input raster
        input: PathBuf,
        /// Output raster
        output: PathBuf,
        /// Target width (>= current width)
        width: usize,
        /// Target height (>= current height)
        height: usize,
    },
    /// Print raster values to terminal (rd_raster_display)
    RasterDisplay {
        /// Input raster
        input: PathBuf,
    },
    /// Print a subregion of a raster to terminal (rd_raster_inspect)
    RasterInspect {
        /// Input raster
        input: PathBuf,
        /// Starting column
        x: usize,
        /// Starting row
        y: usize,
        /// Subregion width
        width: usize,
        /// Subregion height
        height: usize,
    },
    /// Convert TauDEM D8 flow directions to RichDEM D8 (rd_taudem_d8_to_richdem_d8)
    TaudemD8ToRichdemD8 {
        /// Input TauDEM D8 raster
        input: PathBuf,
        /// Output RichDEM D8 raster
        output: PathBuf,
    },
    /// Convert ArcGIS D8 flow directions to RichDEM D8
    ArcD8ToRichdemD8 {
        /// Input ArcGIS D8 raster
        input: PathBuf,
        /// Output RichDEM D8 raster
        output: PathBuf,
    },
}

fn run() -> richdem::Result<()> {
    let cli = Cli::parse();
    let cmdline = std::env::args().collect::<Vec<_>>().join(" ");

    match cli.command {
        Commands::DepressionsFlood {
            input,
            output,
            gradient,
            topology,
            compress,
        } => {
            let topo: Topology = topology.into();
            let mut dyn_arr = load_dynamic(&input)?;
            macro_rules! do_flood {
                ($arr:expr) => {{
                    if gradient {
                        fill_depressions_epsilon($arr, topo)?;
                    } else {
                        fill_depressions($arr, topo);
                    }
                }};
            }
            match &mut dyn_arr {
                DynamicArray2D::U8(a) => do_flood!(a),
                DynamicArray2D::I8(a) => do_flood!(a),
                DynamicArray2D::U16(a) => do_flood!(a),
                DynamicArray2D::I16(a) => do_flood!(a),
                DynamicArray2D::U32(a) => do_flood!(a),
                DynamicArray2D::I32(a) => do_flood!(a),
                DynamicArray2D::U64(a) => do_flood!(a),
                DynamicArray2D::I64(a) => do_flood!(a),
                DynamicArray2D::F32(a) => do_flood!(a),
                DynamicArray2D::F64(a) => do_flood!(a),
            }
            save_dynamic(&dyn_arr, &output, Some(&cmdline), compress)?;
        }

        Commands::DepressionsBreach {
            input,
            output,
            mode,
            max_length,
            max_depth,
            fill_depressions: fill_dep,
            gradient,
            topology,
            compress,
        } => {
            let topo: Topology = topology.into();
            if topo == Topology::D4 && (gradient || mode != CliBreachMode::Complete) {
                return Err(richdem::Error::InvalidArgument(
                    "D4 topology is only supported for complete breaching without --gradient; use --topology d8 for gradient, selective, or constrained breaching".into(),
                ));
            }
            let mut dem = load_raster::<f64>(&input, None)?;
            match mode {
                CliBreachMode::Complete if !gradient => {
                    breach_depressions(&mut dem, topo);
                }
                CliBreachMode::Complete => {
                    lindsay2016(
                        &mut dem,
                        LindsayMode::CompleteBreaching,
                        gradient,
                        fill_dep,
                        max_length,
                        max_depth,
                    )?;
                }
                CliBreachMode::Selective => {
                    lindsay2016(
                        &mut dem,
                        LindsayMode::SelectiveBreaching,
                        gradient,
                        fill_dep,
                        max_length,
                        max_depth,
                    )?;
                }
                CliBreachMode::Constrained => {
                    lindsay2016(
                        &mut dem,
                        LindsayMode::ConstrainedBreaching,
                        gradient,
                        fill_dep,
                        max_length,
                        max_depth,
                    )?;
                }
            }
            save_raster(&dem, &output, Some(&cmdline), compress)?;
        }

        Commands::DepressionsHas { input, topology } => {
            let dem = load_raster::<f64>(&input, None)?;
            let found = has_depressions(&dem, topology.into());
            println!("{found}");
        }

        Commands::DepressionsMask {
            input,
            output,
            topology,
            compress,
        } => {
            let dem = load_raster::<f64>(&input, None)?;
            let mask = pit_mask(&dem, topology.into());
            save_raster(&mask, &output, Some(&cmdline), compress)?;
        }

        Commands::ResolveFlats {
            input,
            output,
            compress,
        } => {
            let mut dem = load_raster::<f64>(&input, None)?;
            resolve_flats_epsilon(&mut dem)?;
            save_raster(&dem, &output, Some(&cmdline), compress)?;
        }

        Commands::D8Flowdirs {
            input,
            output,
            compress,
        } => {
            let dem = load_raster::<f64>(&input, None)?;
            let fds = d8_flow_directions(&dem);
            save_raster(&fds, &output, Some(&cmdline), compress)?;
        }

        Commands::FloodForFlowdirs {
            input,
            output,
            compress,
        } => {
            let dem = load_raster::<f64>(&input, None)?;
            let fds = priority_flood_flowdirs_barnes2014(&dem);
            save_raster(&fds, &output, Some(&cmdline), compress)?;
        }

        Commands::FlowAccumulation {
            input,
            output,
            method,
            exponent,
            from_d8,
            compress,
        } => {
            if from_d8 {
                let fds = load_raster::<u8>(&input, None)?;
                let accum = d8_flow_accum::<u8, i32>(&fds);
                save_raster(&accum, &output, Some(&cmdline), compress)?;
            } else {
                let m = FlowAccumMethod::parse(&method)?;
                let dem = load_raster::<f64>(&input, None)?;
                let mut accum = Array2D::<f64>::make_from_template(&dem, 1.0);
                flow_accumulation_by_method(&dem, &mut accum, m, exponent)?;
                save_raster(&accum, &output, Some(&cmdline), compress)?;
            }
        }

        Commands::TerrainProperty {
            input,
            output,
            attrib,
            zscale,
            compress,
        } => {
            let attr = TerrainAttr::parse(&attrib)?;
            let dem = load_raster::<f64>(&input, None)?;
            let res = terrain_attribute(&dem, attr, zscale);
            save_raster(&res, &output, Some(&cmdline), compress)?;
        }

        Commands::SurfaceArea { input, zscale } => {
            let dem = load_raster::<f64>(&input, None)?;
            let area = dem_surface_area(&dem, zscale)?;
            let planar = (dem.num_data_cells() as f64) * dem.get_cell_area();
            println!("Topographic surface area = {area}");
            println!("Planar surface area      = {planar}");
            println!("Ratio                    = {}", area / planar);
        }

        Commands::DepressionHierarchy {
            input,
            labels_out,
            flowdirs_out,
            topology,
        } => {
            let dem = load_raster::<f64>(&input, None)?;
            let mut labels = Array2D::make_from_template(&dem, NO_DEP);
            labels.set_edges(OCEAN);
            let mut flowdirs = Array2D::make_from_template(&dem, NO_FLOW);
            let dh = get_depression_hierarchy(&dem, &mut labels, &mut flowdirs, topology.into())?;
            println!("{}", format_depression_hierarchy(&dh));
            if let Some(p) = labels_out {
                save_raster(&labels, &p, Some(&cmdline), false)?;
            }
            if let Some(p) = flowdirs_out {
                save_raster(&flowdirs, &p, Some(&cmdline), false)?;
            }
        }

        Commands::FillSpillMerge {
            input,
            output,
            initial_water,
            wtd_in,
            compress,
        } => {
            let dem = load_raster::<f64>(&input, None)?;
            let mut labels = Array2D::make_from_template(&dem, NO_DEP);
            labels.set_edges(OCEAN);
            let mut flowdirs = Array2D::make_from_template(&dem, NO_FLOW);
            let mut dh = get_depression_hierarchy(&dem, &mut labels, &mut flowdirs, Topology::D8)?;
            let mut wtd = match wtd_in {
                Some(p) => load_raster::<f64>(&p, None)?,
                None => Array2D::make_from_template(&dem, initial_water),
            };
            fill_spill_merge(&dem, &labels, &flowdirs, &mut dh, &mut wtd)?;
            save_raster(&wtd, &output, Some(&cmdline), compress)?;
        }

        Commands::Info { input } => {
            let arr = load_dynamic(&input)?;
            println!("File       = {}", input.display());
            println!("Data type  = {}", arr.dtype_name());
            println!("Width      = {}", arr.width());
            println!("Height     = {}", arr.height());
            println!("Shape      = {}x{}", arr.width(), arr.height());
            println!("NoData     = {}", arr.no_data_f64());
            println!("GeoTrans   = {:?}", arr.geotransform());
            if !arr.projection().is_empty() {
                println!("Projection = {}", arr.projection());
            }
            println!("Metadata:");
            for (k, v) in arr.metadata() {
                if k != "PROCESSING_HISTORY" {
                    println!("  {k} = {v}");
                }
            }
            println!("Processing History:");
            println!("-------------------");
            if let Some(hist) = arr.metadata().get("PROCESSING_HISTORY") {
                for line in hist.lines().filter(|l| !l.trim().is_empty()) {
                    println!("{line}");
                }
            }
            println!("-------------------");
        }

        Commands::Compare { first, second } => {
            let a = load_dynamic(&first)?;
            let b = load_dynamic(&second)?;
            if a.width() != b.width() || a.height() != b.height() {
                println!(
                    "Dimensions differ: {}x{} vs {}x{}",
                    a.width(),
                    a.height(),
                    b.width(),
                    b.height()
                );
                return Ok(());
            }
            if a.no_data_f64() != b.no_data_f64() {
                println!("NoData differs: {} vs {}", a.no_data_f64(), b.no_data_f64());
            }
            if a.geotransform() != b.geotransform() {
                println!(
                    "Geotransform differs: {:?} vs {:?}",
                    a.geotransform(),
                    b.geotransform()
                );
            }
            if a.projection() != b.projection() {
                println!("Projection differs");
            }
            let fa = a.to_f64();
            let fb = b.to_f64();
            let mut max_diff = 0.0f64;
            let mut min_diff = f64::INFINITY;
            let mut sum_abs = 0.0f64;
            let mut sum_sq = 0.0f64;
            let mut count = 0usize;
            let mut validity_mismatch = 0usize;
            for i in 0..fa.size() {
                let va = fa.get_i(i);
                let vb = fb.get_i(i);
                let a_missing = fa.is_no_data_i(i) || va.is_nan();
                let b_missing = fb.is_no_data_i(i) || vb.is_nan();
                match (a_missing, b_missing) {
                    (true, true) => continue,
                    (true, false) | (false, true) => {
                        validity_mismatch += 1;
                        continue;
                    }
                    (false, false) => {}
                }
                let d = (va - vb).abs();
                max_diff = max_diff.max(d);
                min_diff = min_diff.min(d);
                sum_abs += d;
                sum_sq += d * d;
                count += 1;
            }
            if count == 0 {
                min_diff = 0.0;
            }
            let avg = if count > 0 {
                sum_abs / count as f64
            } else {
                0.0
            };
            let rms = if count > 0 {
                (sum_sq / count as f64).sqrt()
            } else {
                0.0
            };
            if validity_mismatch > 0 {
                println!("Validity mismatch cells: {validity_mismatch}");
            }
            println!("Absolute Max difference: {max_diff:10.6}");
            println!("Absolute Min difference: {min_diff:10.6}");
            println!("Absolute Avg difference: {avg:10.6}");
            println!("RMS difference:          {rms:10.6}");
        }

        Commands::Geotransform {
            input,
            output,
            in_place,
            set,
        } => {
            let mut arr = load_dynamic(&input)?;
            if let Some(vals) = set {
                let out = match (output, in_place) {
                    (Some(p), _) => p,
                    (None, true) => input,
                    (None, false) => {
                        return Err(richdem::Error::InvalidArgument(
                            "setting geotransform requires --output <PATH> or --in-place".into(),
                        ));
                    }
                };
                let gt: [f64; 6] = vals.try_into().map_err(|_| {
                    richdem::Error::InvalidArgument("expected 6 geotransform values".into())
                })?;
                match &mut arr {
                    DynamicArray2D::U8(a) => a.geotransform = gt,
                    DynamicArray2D::I8(a) => a.geotransform = gt,
                    DynamicArray2D::U16(a) => a.geotransform = gt,
                    DynamicArray2D::I16(a) => a.geotransform = gt,
                    DynamicArray2D::U32(a) => a.geotransform = gt,
                    DynamicArray2D::I32(a) => a.geotransform = gt,
                    DynamicArray2D::U64(a) => a.geotransform = gt,
                    DynamicArray2D::I64(a) => a.geotransform = gt,
                    DynamicArray2D::F32(a) => a.geotransform = gt,
                    DynamicArray2D::F64(a) => a.geotransform = gt,
                }
                save_dynamic(&arr, &out, Some(&cmdline), false)?;
            } else {
                println!("{:?}", arr.geotransform());
            }
        }

        Commands::NoData {
            input,
            output,
            in_place,
            set,
        } => {
            let mut arr = load_dynamic(&input)?;
            if let Some(nd_str) = set {
                let out = match (output, in_place) {
                    (Some(p), _) => p,
                    (None, true) => input,
                    (None, false) => {
                        return Err(richdem::Error::InvalidArgument(
                            "setting NoData requires --output <PATH> or --in-place".into(),
                        ));
                    }
                };
                fn parse_int_nodata<T: std::str::FromStr>(s: &str, name: &str) -> richdem::Result<T>
                where
                    T::Err: std::fmt::Display,
                {
                    s.trim().parse::<T>().map_err(|e| {
                        richdem::Error::InvalidArgument(format!(
                            "NoData value '{s}' is not representable as {name}: {e}"
                        ))
                    })
                }
                match &mut arr {
                    DynamicArray2D::U8(a) => a.set_no_data(parse_int_nodata::<u8>(&nd_str, "u8")?),
                    DynamicArray2D::I8(a) => a.set_no_data(parse_int_nodata::<i8>(&nd_str, "i8")?),
                    DynamicArray2D::U16(a) => {
                        a.set_no_data(parse_int_nodata::<u16>(&nd_str, "u16")?)
                    }
                    DynamicArray2D::I16(a) => {
                        a.set_no_data(parse_int_nodata::<i16>(&nd_str, "i16")?)
                    }
                    DynamicArray2D::U32(a) => {
                        a.set_no_data(parse_int_nodata::<u32>(&nd_str, "u32")?)
                    }
                    DynamicArray2D::I32(a) => {
                        a.set_no_data(parse_int_nodata::<i32>(&nd_str, "i32")?)
                    }
                    DynamicArray2D::U64(a) => {
                        a.set_no_data(parse_int_nodata::<u64>(&nd_str, "u64")?)
                    }
                    DynamicArray2D::I64(a) => {
                        a.set_no_data(parse_int_nodata::<i64>(&nd_str, "i64")?)
                    }
                    DynamicArray2D::F32(a) => {
                        let v: f32 = nd_str.trim().parse().map_err(|e| {
                            richdem::Error::InvalidArgument(format!(
                                "invalid f32 NoData '{nd_str}': {e}"
                            ))
                        })?;
                        a.set_no_data(v);
                    }
                    DynamicArray2D::F64(a) => {
                        let v: f64 = nd_str.trim().parse().map_err(|e| {
                            richdem::Error::InvalidArgument(format!(
                                "invalid f64 NoData '{nd_str}': {e}"
                            ))
                        })?;
                        a.set_no_data(v);
                    }
                }
                save_dynamic(&arr, &out, Some(&cmdline), false)?;
            } else {
                println!("{}", arr.no_data_f64());
            }
        }

        Commands::Projection {
            input,
            output,
            in_place,
            set,
            from,
        } => {
            let mut arr = load_dynamic(&input)?;
            let new_proj = if let Some(src_path) = from {
                let src = load_dynamic(&src_path)?;
                Some(src.projection().to_string())
            } else {
                set
            };
            if let Some(p) = new_proj {
                let out = match (output, in_place) {
                    (Some(path), _) => path,
                    (None, true) => input,
                    (None, false) => {
                        return Err(richdem::Error::InvalidArgument(
                            "setting projection requires --output <PATH> or --in-place".into(),
                        ));
                    }
                };
                match &mut arr {
                    DynamicArray2D::U8(a) => a.projection = p,
                    DynamicArray2D::I8(a) => a.projection = p,
                    DynamicArray2D::U16(a) => a.projection = p,
                    DynamicArray2D::I16(a) => a.projection = p,
                    DynamicArray2D::U32(a) => a.projection = p,
                    DynamicArray2D::I32(a) => a.projection = p,
                    DynamicArray2D::U64(a) => a.projection = p,
                    DynamicArray2D::I64(a) => a.projection = p,
                    DynamicArray2D::F32(a) => a.projection = p,
                    DynamicArray2D::F64(a) => a.projection = p,
                }
                save_dynamic(&arr, &out, Some(&cmdline), false)?;
            } else {
                println!("{}", arr.projection());
            }
        }

        Commands::ProcessingHistory { input } => {
            let arr = load_dynamic(&input)?;
            if let Some(hist) = arr.metadata().get("PROCESSING_HISTORY") {
                println!("{hist}");
            }
        }

        Commands::Hist { input } => {
            let arr = load_raster::<f64>(&input, None)?;
            let mut vals = arr.data().to_vec();
            vals.sort_by(f64::total_cmp);
            println!("NoData: {}", arr.no_data());
            let mut i = 0;
            while i < vals.len() {
                let v = vals[i];
                let mut j = i + 1;
                while j < vals.len()
                    && ((v.is_nan() && vals[j].is_nan()) || (!v.is_nan() && vals[j] == v))
                {
                    j += 1;
                }
                let k = format!("{v}");
                let c = j - i;
                println!("{k:>20} {c:>20}");
                i = j;
            }
        }

        Commands::LoopCheck { input } => {
            let fds = load_raster::<u8>(&input, None)?;
            let loops = find_flow_loops(&fds);
            if loops.is_empty() {
                println!("No flow loops detected.");
            } else {
                println!("Detected {} flow loop(s):", loops.len());
                for (x, y) in loops {
                    println!("  ({x}, {y})");
                }
            }
        }

        Commands::ExpandDimensions {
            input,
            output,
            width,
            height,
        } => {
            let mut arr = load_dynamic(&input)?;
            match &mut arr {
                DynamicArray2D::U8(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::I8(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::U16(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::I16(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::U32(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::I32(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::U64(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::I64(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::F32(a) => a.expand(width, height, a.no_data())?,
                DynamicArray2D::F64(a) => a.expand(width, height, a.no_data())?,
            }
            save_dynamic(&arr, &output, Some(&cmdline), false)?;
        }

        Commands::RasterDisplay { input } => {
            let arr = load_raster::<f64>(&input, None)?;
            print!("{}", arr.format_grid());
        }

        Commands::RasterInspect {
            input,
            x,
            y,
            width,
            height,
        } => {
            let arr = load_raster::<f64>(&input, None)?;
            let x_end = (x + width).min(arr.width());
            let y_end = (y + height).min(arr.height());
            for row in y..y_end {
                for col in x..x_end {
                    if col > x {
                        print!(" ");
                    }
                    print!("{:>8.3}", arr.get(col, row));
                }
                println!();
            }
        }

        Commands::TaudemD8ToRichdemD8 { input, output } => {
            let fds = load_raster::<u8>(&input, None)?;
            let out = convert_taudem_d8_to_richdem_d8(&fds)?;
            save_raster(&out, &output, Some(&cmdline), false)?;
        }

        Commands::ArcD8ToRichdemD8 { input, output } => {
            let fds = load_raster::<u8>(&input, None)?;
            let out = convert_arc_flowdirs_to_richdem_d8(&fds)?;
            save_raster(&out, &output, Some(&cmdline), false)?;
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("E {e}");
            ExitCode::FAILURE
        }
    }
}
