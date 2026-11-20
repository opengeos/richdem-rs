//! Integration test suite ported from RichDEM's `tests/tests.cpp` plus additional
//! parity and `terrio` geospatial I/O tests.

use std::path::PathBuf;

use richdem::*;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("test_data")
}

#[test]
fn test_array2d_works() {
    let mut arr = Array2D::<f32>::new(7, 11, 0.0);
    assert_eq!(arr.width(), 7);
    assert_eq!(arr.height(), 11);
    assert_eq!(arr.size(), 77);

    assert_eq!(arr.i_to_xy(0), (0, 0));
    assert_eq!(arr.i_to_xy(arr.size() - 1), (6, 10));
    assert_eq!(arr.xy_to_i(2, 1), 9);
    assert_eq!(arr.xy_to_i(6, 2), 20);

    let mut val = 0.0f32;
    for y in 0..arr.height() {
        for x in 0..arr.width() {
            arr.set(x, y, val);
            val += 1.0;
        }
    }
    assert_eq!(arr.get(2, 1), 9.0);
    assert_eq!(arr.get(6, 2), 20.0);
    assert_eq!(arr.get(0, 0), 0.0);

    // Resize test
    let mut arr2 = Array2D::<f32>::default();
    arr2.resize(3, 5, 4.0);
    assert_eq!(arr2.width(), 3);
    assert_eq!(arr2.height(), 5);
    assert_eq!(arr2.size(), 15);
    assert_eq!(arr2.get(2, 1), 4.0);
    assert_eq!(arr2.get(0, 0), 4.0);

    // Rows constructor
    let arr3 = Array2D::from_rows(&[
        &[1.0f32, 2.0, 3.0],
        &[4.0, 5.0, 6.0],
        &[7.0, 8.0, 9.0],
        &[10.0, 11.0, 12.0],
    ])
    .unwrap();
    assert_eq!(arr3.width(), 3);
    assert_eq!(arr3.height(), 4);
    assert_eq!(arr3.get(0, 0), 1.0);
    assert_eq!(arr3.get(2, 0), 3.0);
    assert_eq!(arr3.get(0, 2), 7.0);
    assert_eq!(arr3.get(2, 3), 12.0);
}

#[test]
fn test_padding_on_load() {
    let p = test_data_dir().join("ones_block.dem");
    let temp = load_raster_padded::<i32>(&p, 1, 1).unwrap();
    for y in 0..temp.height() {
        for x in 0..temp.width() {
            if x == 0 || y == 0 || x == temp.width() - 1 || y == temp.height() - 1 {
                assert_eq!(temp.get(x, y), 0);
            } else {
                assert_eq!(temp.get(x, y), 1);
            }
        }
    }
}

#[test]
fn test_flow_accum_fixtures() {
    let dir = test_data_dir().join("flow_accum");
    let mut checked = 0usize;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("d8") {
            let fds = Array2D::<u8>::load_ascii_grid(&path).unwrap();
            let out_path = path.with_extension("out");
            let correct_ans = Array2D::<i32>::load_ascii_grid(&out_path).unwrap();
            let my_ans = d8_flow_accum::<u8, i32>(&fds);
            assert_eq!(
                correct_ans,
                my_ans,
                "mismatch on fixture {}",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 20,
        "expected >=20 flow_accum fixtures, got {checked}"
    );
}

#[test]
fn test_grid_cell_zk_low_pq() {
    // Elevation ordering
    let mut pq = GridCellZkLowPq::<i32>::new();
    pq.emplace(0, 0, 0);
    pq.emplace(1, 0, 1);
    pq.emplace(2, 0, 2);
    pq.emplace(3, 0, 3);
    assert_eq!(pq.pop().unwrap().x, 0);
    assert_eq!(pq.pop().unwrap().x, 1);
    assert_eq!(pq.pop().unwrap().x, 2);
    assert_eq!(pq.pop().unwrap().x, 3);
    assert!(pq.is_empty());

    // Insertion ordering (FIFO)
    pq.emplace(0, 0, 0);
    pq.emplace(1, 0, 0);
    pq.emplace(2, 0, 0);
    pq.emplace(3, 0, 0);
    assert_eq!(pq.pop().unwrap().x, 0);
    assert_eq!(pq.pop().unwrap().x, 1);
    assert_eq!(pq.pop().unwrap().x, 2);
    assert_eq!(pq.pop().unwrap().x, 3);
    assert!(pq.is_empty());

    // Mixed ordering
    pq.emplace(0, 0, 0);
    pq.emplace(1, 0, 1);
    pq.emplace(2, 0, 1);
    pq.emplace(3, 0, 2);
    assert_eq!(pq.pop().unwrap().x, 0);
    assert_eq!(pq.pop().unwrap().x, 1);
    assert_eq!(pq.pop().unwrap().x, 2);
    assert_eq!(pq.pop().unwrap().x, 3);
    assert!(pq.is_empty());
}

#[test]
fn test_grid_cell_zk_high_pq() {
    // Elevation ordering
    let mut pq = GridCellZkHighPq::<i32>::new();
    pq.emplace(0, 0, 0);
    pq.emplace(1, 0, 1);
    pq.emplace(2, 0, 2);
    pq.emplace(3, 0, 3);
    assert_eq!(pq.pop().unwrap().x, 0);
    assert_eq!(pq.pop().unwrap().x, 1);
    assert_eq!(pq.pop().unwrap().x, 2);
    assert_eq!(pq.pop().unwrap().x, 3);
    assert!(pq.is_empty());

    // Insertion ordering (LIFO)
    pq.emplace(0, 0, 0);
    pq.emplace(1, 0, 0);
    pq.emplace(2, 0, 0);
    pq.emplace(3, 0, 0);
    assert_eq!(pq.pop().unwrap().x, 3);
    assert_eq!(pq.pop().unwrap().x, 2);
    assert_eq!(pq.pop().unwrap().x, 1);
    assert_eq!(pq.pop().unwrap().x, 0);
    assert!(pq.is_empty());

    // Mixed ordering
    pq.emplace(0, 0, 0);
    pq.emplace(1, 0, 1);
    pq.emplace(2, 0, 1);
    pq.emplace(3, 0, 2);
    assert_eq!(pq.pop().unwrap().x, 0);
    assert_eq!(pq.pop().unwrap().x, 2);
    assert_eq!(pq.pop().unwrap().x, 1);
    assert_eq!(pq.pop().unwrap().x, 3);
    assert!(pq.is_empty());
}

#[test]
fn test_depression_filling_fixtures() {
    let dir = test_data_dir().join("depressions");
    let elevation_orig = Array2D::<i32>::load_ascii_grid(dir.join("testdem1.dem")).unwrap();
    let expected_all = Array2D::<i32>::load_ascii_grid(dir.join("testdem1.all.out")).unwrap();

    assert!(has_depressions(&elevation_orig, Topology::D8));

    {
        let mut elev = elevation_orig.clone();
        priority_flood_original(&mut elev, Topology::D8);
        assert_eq!(elev, expected_all);
        assert!(!has_depressions(&elev, Topology::D8));
    }
    {
        let mut elev = elevation_orig.clone();
        priority_flood_barnes2014(&mut elev, Topology::D8);
        assert_eq!(elev, expected_all);
    }
    {
        let mut elev = elevation_orig.clone();
        priority_flood_zhou2016(&mut elev);
        assert_eq!(elev, expected_all);
    }
    {
        let mut elev = elevation_orig.clone();
        priority_flood_wei2018(&mut elev);
        assert_eq!(elev, expected_all);
    }
    {
        let mut elev = elevation_orig.clone();
        fill_depressions(&mut elev, Topology::D8);
        assert_eq!(elev, expected_all);
    }
    {
        let mut elev = elevation_orig.clone();
        priority_flood_barnes2014_max_dep(&mut elev, 1, Topology::D8);
        let expected_1 = Array2D::<i32>::load_ascii_grid(dir.join("testdem1.1.out")).unwrap();
        assert_eq!(elev, expected_1);
    }
    {
        let mut elev = elevation_orig.clone();
        priority_flood_barnes2014_max_dep(&mut elev, 2, Topology::D8);
        let expected_2 = Array2D::<i32>::load_ascii_grid(dir.join("testdem1.2.out")).unwrap();
        assert_eq!(elev, expected_2);
    }

    // Pit mask check
    let mask = pit_mask(&elevation_orig, Topology::D8);
    assert!(mask.count_val(1) > 0);
}

#[test]
fn test_depression_breaching_fixtures() {
    let dir = test_data_dir().join("breaching");
    let elevation_orig = Array2D::<i32>::load_ascii_grid(dir.join("testdem1.dem")).unwrap();

    {
        let mut elev = elevation_orig.clone();
        complete_breaching_lindsay2016(&mut elev, Topology::D8);
        let expected = Array2D::<i32>::load_ascii_grid(dir.join("testdem1.complete.out")).unwrap();
        assert_eq!(elev, expected);
    }
    {
        let mut elev = elevation_orig.clone();
        lindsay2016(
            &mut elev,
            LindsayMode::SelectiveBreaching,
            false,
            false,
            2,
            9999,
        )
        .unwrap();
        let expected =
            Array2D::<i32>::load_ascii_grid(dir.join("testdem1.selective-len2-depth9999.out"))
                .unwrap();
        assert_eq!(elev, expected);
    }
    {
        let mut elev = elevation_orig.clone();
        lindsay2016(
            &mut elev,
            LindsayMode::SelectiveBreaching,
            false,
            false,
            4,
            9999,
        )
        .unwrap();
        let expected =
            Array2D::<i32>::load_ascii_grid(dir.join("testdem1.selective-len4-depth9999.out"))
                .unwrap();
        assert_eq!(elev, expected);
    }
    {
        let mut elev = elevation_orig.clone();
        lindsay2016(
            &mut elev,
            LindsayMode::SelectiveBreaching,
            false,
            false,
            4,
            2,
        )
        .unwrap();
        let expected =
            Array2D::<i32>::load_ascii_grid(dir.join("testdem1.selective-len4-depth2.out"))
                .unwrap();
        assert_eq!(elev, expected);
    }
    {
        let mut elev = elevation_orig.clone();
        lindsay2016(
            &mut elev,
            LindsayMode::SelectiveBreaching,
            false,
            true,
            4,
            2,
        )
        .unwrap();
        let expected =
            Array2D::<i32>::load_ascii_grid(dir.join("testdem1.selective-len4-depth2-filldep.out"))
                .unwrap();
        assert_eq!(elev, expected);
    }
    {
        let mut elev = elevation_orig.clone();
        lindsay2016(
            &mut elev,
            LindsayMode::SelectiveBreaching,
            false,
            false,
            4,
            8,
        )
        .unwrap();
        let expected =
            Array2D::<i32>::load_ascii_grid(dir.join("testdem1.selective-len4-depth8.out"))
                .unwrap();
        assert_eq!(elev, expected);
    }
    {
        let mut elev = elevation_orig.clone();
        lindsay2016(
            &mut elev,
            LindsayMode::ConstrainedBreaching,
            false,
            false,
            4,
            3,
        )
        .unwrap();
        let expected =
            Array2D::<i32>::load_ascii_grid(dir.join("testdem1.constrained-len4-depth3.out"))
                .unwrap();
        assert_eq!(elev, expected);
    }
}

#[test]
fn test_bucket_fill() {
    let check_raster = Array2D::from_rows(&[
        &[1, 1, 1, 1, 1, 1, 1],
        &[1, 1, 2, 1, 1, 2, 1],
        &[1, 1, 2, 1, 2, 1, 1],
        &[1, 1, 2, 2, 1, 1, 1],
        &[1, 1, 1, 1, 1, 1, 1],
    ])
    .unwrap();
    let mut set_raster = Array2D::<i32>::new(check_raster.width(), check_raster.height(), 0);
    let mut seeds = vec![set_raster.xy_to_i(2, 1)];
    bucket_fill(
        &check_raster,
        &mut set_raster,
        2,
        4,
        &mut seeds,
        Topology::D8,
    )
    .unwrap();

    let good = Array2D::from_rows(&[
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 0, 4, 0, 0, 4, 0],
        &[0, 0, 4, 0, 4, 0, 0],
        &[0, 0, 4, 4, 0, 0, 0],
        &[0, 0, 0, 0, 0, 0, 0],
    ])
    .unwrap();
    assert_eq!(set_raster, good);

    // Edges
    let check_edges = Array2D::from_rows(&[
        &[1, 1, 1, 1, 1, 2, 1],
        &[1, 1, 2, 1, 2, 1, 1],
        &[1, 1, 2, 1, 1, 2, 1],
        &[1, 1, 2, 2, 1, 1, 1],
        &[1, 2, 1, 1, 1, 1, 1],
    ])
    .unwrap();
    let mut set_edges = Array2D::<i32>::new(check_edges.width(), check_edges.height(), 0);
    bucket_fill_from_edges(&check_edges, &mut set_edges, 2, 4, Topology::D8).unwrap();
    let good_edges = Array2D::from_rows(&[
        &[0, 0, 0, 0, 0, 4, 0],
        &[0, 0, 4, 0, 4, 0, 0],
        &[0, 0, 4, 0, 0, 4, 0],
        &[0, 0, 4, 4, 0, 0, 0],
        &[0, 4, 0, 0, 0, 0, 0],
    ])
    .unwrap();
    assert_eq!(set_edges, good_edges);
}

#[test]
fn test_directions_match_expectations() {
    assert_eq!(Topology::D8.dx(), &[0, -1, -1, 0, 1, 1, 1, 0, -1]);
    assert_eq!(Topology::D8.dy(), &[0, 0, -1, -1, -1, 0, 1, 1, 1]);
    assert_eq!(Topology::D8.nmax(), 8);

    assert_eq!(Topology::D4.dx(), &[0, -1, 0, 1, 0]);
    assert_eq!(Topology::D4.dy(), &[0, 0, -1, 0, 1]);
    assert_eq!(Topology::D4.nmax(), 4);
}

#[test]
fn test_flow_accumulation_from_d8() {
    let d8s = Array2D::from_rows(&[
        &[1, 1, 1, 1, 1, 1, 1],
        &[1, 1, 1, 1, 1, 1, 1],
        &[1, 1, 1, 1, 1, 1, 1],
        &[1, 1, 1, 1, 1, 1, 1],
        &[1, 1, 1, 1, 1, 1, 1],
    ])
    .unwrap();
    let mut expected = Array2D::from_rows(&[
        &[0u32, 0, 0, 0, 0, 0, 0],
        &[0, 5, 4, 3, 2, 1, 0],
        &[0, 5, 4, 3, 2, 1, 0],
        &[0, 5, 4, 3, 2, 1, 0],
        &[0, 0, 0, 0, 0, 0, 0],
    ])
    .unwrap();
    expected.set_no_data(ACCUM_NO_DATA as u32);
    assert_eq!(flow_accumulation_from_d8(&d8s), expected);

    let snake = Array2D::from_rows(&[
        &[0, 0, 0, 0, 0, 0, 0],
        &[0, 7, 1, 7, 1, 7, 0],
        &[0, 7, 3, 7, 3, 7, 0],
        &[0, 1, 3, 1, 3, 1, 0],
        &[0, 0, 0, 0, 0, 0, 0],
    ])
    .unwrap();
    let mut expected_snake = Array2D::from_rows(&[
        &[0u32, 0, 0, 0, 0, 0, 0],
        &[0, 13, 12, 7, 6, 1, 0],
        &[0, 14, 11, 8, 5, 2, 0],
        &[0, 15, 10, 9, 4, 3, 0],
        &[0, 0, 0, 0, 0, 0, 0],
    ])
    .unwrap();
    expected_snake.set_no_data(ACCUM_NO_DATA as u32);
    assert_eq!(flow_accumulation_from_d8(&snake), expected_snake);
}

#[test]
fn test_all_flow_accumulation_methods_on_perlin() {
    let mut dem = generate_perlin_terrain(32, 42);
    priority_flood_epsilon_barnes2014(&mut dem, Topology::D8).unwrap();

    for method in [
        FlowAccumMethod::Tarboton,
        FlowAccumMethod::Dinf,
        FlowAccumMethod::Quinn,
        FlowAccumMethod::FairfieldLeymarieD8,
        FlowAccumMethod::FairfieldLeymarieD4,
        FlowAccumMethod::Rho8,
        FlowAccumMethod::Rho4,
        FlowAccumMethod::OCallaghanD8,
        FlowAccumMethod::OCallaghanD4,
        FlowAccumMethod::D8,
        FlowAccumMethod::D4,
    ] {
        let mut accum = Array2D::<f64>::make_from_template(&dem, 1.0);
        flow_accumulation_by_method(&dem, &mut accum, method, None).unwrap();
        assert!(accum.max().unwrap() >= 1.0, "method {method:?} failed");
    }

    for method in [FlowAccumMethod::Holmgren, FlowAccumMethod::Freeman] {
        let mut accum = Array2D::<f64>::make_from_template(&dem, 1.0);
        flow_accumulation_by_method(&dem, &mut accum, method, Some(0.8)).unwrap();
        assert!(accum.max().unwrap() >= 1.0, "method {method:?} failed");
    }
}

#[test]
fn test_terrain_attributes_and_surface_area() {
    // Tilted plane z = 2*x + 3*y with 1x1 cell size
    let mut dem = Array2D::<f64>::new(10, 10, 0.0);
    dem.set_no_data(-9999.0);
    for y in 0..10 {
        for x in 0..10 {
            dem.set(x, y, 2.0 * (x as f64) + 3.0 * (y as f64));
        }
    }

    let slope_rr = ta_slope_riserun(&dem, 1.0);
    let expected_rr = (2.0f64.hypot(3.0)) as f32;
    assert!((slope_rr.get(5, 5) - expected_rr).abs() < 1e-4);

    let slope_pct = ta_slope_percentage(&dem, 1.0);
    assert!((slope_pct.get(5, 5) - expected_rr * 100.0).abs() < 1e-2);

    let slope_deg = ta_slope_degrees(&dem, 1.0);
    let slope_rad = ta_slope_radians(&dem, 1.0);
    assert!((slope_deg.get(5, 5) - slope_rad.get(5, 5).to_degrees()).abs() < 1e-4);

    let aspect = ta_aspect(&dem, 1.0);
    assert!(aspect.get(5, 5) >= 0.0 && aspect.get(5, 5) <= 360.0);

    // Flat plane has zero curvature
    let curv = ta_curvature(&dem, 1.0);
    assert!(curv.get(5, 5).abs() < 1e-4);

    // Surface area of tilted plane > planar area (100.0)
    let area = dem_surface_area(&dem, 1.0).unwrap();
    assert!(area > 100.0);

    // SPI and CTI
    let accum = Array2D::<f64>::make_from_template(&dem, 10.0);
    let spi = ta_spi(&accum, &slope_rr).unwrap();
    let cti = ta_cti(&accum, &slope_rr).unwrap();
    assert!(spi.get(5, 5).is_finite());
    assert!(cti.get(5, 5).is_finite());
}

#[test]
fn test_flat_resolution_and_loop_check() {
    let mut dem = Array2D::from_rows(&[
        &[10.0f64, 10.0, 10.0, 10.0, 10.0],
        &[10.0, 5.0, 5.0, 5.0, 10.0],
        &[10.0, 5.0, 5.0, 5.0, 10.0],
        &[10.0, 5.0, 5.0, 5.0, 10.0],
        &[10.0, 10.0, 1.0, 10.0, 10.0],
    ])
    .unwrap();
    dem.set_no_data(-9999.0);

    let flats_before = find_flats(&dem);
    assert!(flats_before.count_val(IS_A_FLAT) > 0);

    resolve_flats_epsilon(&mut dem).unwrap();
    let flats_after = find_flats(&dem);
    assert_eq!(flats_after.count_val(IS_A_FLAT), 0);

    let fds = d8_flow_directions(&dem);
    assert!(find_flow_loops(&fds).is_empty());

    // Also test D-infinity flat resolution & upslope area
    let dinf_fds = resolve_flats_barnes_dinf(&dem);
    let dinf_area = dinf_upslope_area::<f32, f64>(&dinf_fds);
    assert!(dinf_area.max().unwrap() > 1.0);
}

#[test]
fn test_flowdir_conversions() {
    let arc = Array2D::from_rows(&[&[0u8, 1, 2, 4, 8, 16, 32, 64, 128]]).unwrap();
    let rd = convert_arc_flowdirs_to_richdem_d8(&arc).unwrap();
    assert_eq!(rd.data(), &[0, 5, 6, 7, 8, 1, 2, 3, 4]);

    let td = Array2D::from_rows(&[&[0u8, 1, 2, 3, 4, 5, 6, 7, 8]]).unwrap();
    let rd2 = convert_taudem_d8_to_richdem_d8(&td).unwrap();
    assert_eq!(rd2.data(), &[0, 5, 4, 3, 2, 1, 8, 7, 6]);
}

#[test]
fn test_terrio_geotiff_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test_dem.tif");

    let mut dem = generate_perlin_terrain(24, 123);
    dem.geotransform = [500_000.0, 10.0, 0.0, 4_500_000.0, 0.0, -10.0];
    dem.projection = "EPSG:32613".to_string();
    dem.set_no_data(-9999.0);
    dem.set(0, 0, -9999.0);

    save_raster(&dem, &path, Some("unit_test_roundtrip"), true).unwrap();

    let loaded = load_raster::<f64>(&path, None).unwrap();
    assert_eq!(loaded.width(), 24);
    assert_eq!(loaded.height(), 24);
    assert_eq!(loaded.no_data(), -9999.0);
    assert_eq!(loaded.geotransform, dem.geotransform);
    assert!(!loaded.projection.is_empty());
    assert!(
        loaded
            .metadata
            .get("PROCESSING_HISTORY")
            .is_some_and(|h| h.contains("unit_test_roundtrip"))
    );
    for i in 0..dem.size() {
        assert!((loaded.get_i(i) - dem.get_i(i)).abs() < 1e-12);
    }
}
