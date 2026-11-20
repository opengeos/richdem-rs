//! Fill-Spill-Merge and Depression Hierarchy test suite ported from RichDEM's `tests/fsm_tests.cpp`.

use std::collections::HashSet;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use richdem::*;

fn max_array_diff(a: &Array2D<f64>, b: &Array2D<f64>) -> f64 {
    assert_eq!(a.width(), b.width());
    assert_eq!(a.height(), b.height());
    let mut max_diff = 0.0f64;
    for i in 0..a.size() {
        let av = a.get_i(i);
        let bv = b.get_i(i);
        assert!(
            av.is_finite() && bv.is_finite(),
            "non-finite value at cell {i}: {av} vs {bv}"
        );
        max_diff = max_diff.max((av - bv).abs());
    }
    max_diff
}

fn random_terrain(rng: &mut StdRng, min_size: usize, max_size: usize) -> Array2D<f64> {
    let size = rng.gen_range(min_size..=max_size);
    let seed: u32 = rng.r#gen();
    generate_perlin_terrain(size, seed)
}

fn random_integer_terrain(rng: &mut StdRng, min_size: usize, max_size: usize) -> Array2D<f64> {
    let size = rng.gen_range(min_size..=max_size);
    let seed: u32 = rng.r#gen();
    let mut dem = generate_perlin_terrain(size, seed);
    for v in dem.data_mut().iter_mut() {
        *v = ((*v * 100.0) as i32) as f64;
    }
    dem
}

#[test]
fn test_depression_volume() {
    assert_eq!(depression_volume(2.0, 5, 10.0), 0.0);
    assert_eq!(depression_volume(3.0, 5, 10.0), 5.0);
    assert_eq!(depression_volume(4.0, 5, 10.0), 10.0);
}

#[test]
fn test_determine_water_level() {
    // Depression volume exactly equals water volume
    let mut sill_wtd = -2.0;
    let wl = determine_water_level(&mut sill_wtd, 10.0, 4.0, 5, 10.0);
    assert_eq!(sill_wtd, -2.0);
    assert_eq!(wl, 4.0);

    // Water volume is less than depression volume
    let mut sill_wtd = -2.0;
    let wl = determine_water_level(&mut sill_wtd, 8.0, 4.0, 5, 10.0);
    assert_eq!(sill_wtd, -2.0);
    assert_eq!(wl, 18.0 / 5.0);

    // Water volume is greater than depression volume
    let mut sill_wtd = -2.0;
    let wl = determine_water_level(&mut sill_wtd, 12.0, 4.0, 5, 10.0);
    assert_eq!(sill_wtd, 0.0);
    assert_eq!(wl, 4.0);
}

#[test]
fn test_move_water_into_pits_1() {
    let topo = Array2D::from_rows(&[
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0],
        &[-9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, -9.0],
        &[-9.0, 9.0, 8.0, 8.0, 8.0, 8.0, 7.0, 6.0, 9.0, -9.0],
        &[-9.0, 9.0, 8.0, 7.0, 8.0, 7.0, 6.0, 5.0, 9.0, -9.0],
        &[-9.0, 9.0, 8.0, 7.0, 8.0, 6.0, 5.0, 4.0, 9.0, -9.0],
        &[-9.0, 9.0, 8.0, 8.0, 8.0, 5.0, 4.0, 3.0, 9.0, -9.0],
        &[-9.0, 9.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 9.0, -9.0],
        &[-9.0, 9.0, 7.0, 6.0, 5.0, 4.0, 3.0, 1.0, 9.0, -9.0],
        &[-9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, 9.0, -9.0],
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0],
    ])
    .unwrap();

    let mut label = Array2D::<DhLabel>::new(topo.width(), topo.height(), NO_DEP);
    label.set_edges(OCEAN);
    let mut flowdirs = Array2D::<Flowdir>::new(topo.width(), topo.height(), NO_FLOW);
    let mut wtd = Array2D::<f64>::new(topo.width(), topo.height(), 1.0);

    let mut dh = get_depression_hierarchy(&topo, &mut label, &mut flowdirs, Topology::D8).unwrap();

    move_water_into_pits(&topo, &label, &flowdirs, &mut dh, &mut wtd);

    assert!(wtd.data().iter().all(|&v| v == 0.0));
    assert_eq!(dh[0].water_vol, 64.0);
    assert_eq!(dh[1].water_vol, 30.0);
    assert_eq!(dh[2].water_vol, 6.0);

    assert_eq!(dh[0].parent, NO_VALUE);
    assert_eq!(dh[1].parent, 3);
    assert_eq!(dh[2].parent, 3);
    assert_eq!(dh[3].parent, 0);

    assert!(dh[0].dep_vol.is_nan());
    assert_eq!(dh[1].dep_vol, 73.0);
    assert_eq!(dh[2].dep_vol, 2.0);
    assert_eq!(dh[3].dep_vol, 111.0);
}

#[test]
fn test_backfill_depression() {
    let topo = Array2D::from_rows(&[
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0],
        &[-9.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 1.0, 1.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 1.0, 3.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 2.0, 1.0, 4.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 1.0, 1.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 6.0, 6.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, -9.0],
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0],
    ])
    .unwrap();

    let mut wtd = Array2D::<f64>::new(topo.width(), topo.height(), 0.0);
    let cells_affected: Vec<FlatIdx> = [
        (4, 2),
        (5, 2),
        (4, 3),
        (5, 3),
        (4, 4),
        (5, 4),
        (4, 5),
        (5, 5),
    ]
    .iter()
    .map(|&(x, y)| topo.xy_to_i(x, y) as FlatIdx)
    .collect();

    backfill_depression(4.0, &topo, &mut wtd, &cells_affected);

    let wtd_good = Array2D::from_rows(&[
        &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 3.0, 3.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 3.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 3.0, 3.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ])
    .unwrap();
    assert_eq!(wtd, wtd_good);
}

#[test]
fn test_fill_depressions_fsm_cases() {
    let topo = Array2D::from_rows(&[
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0],
        &[-9.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 1.0, 1.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 3.0, 3.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 2.0, 1.0, 4.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 1.0, 1.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 6.0, 6.0, 6.0, 6.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 6.0, -9.0],
        &[-9.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, -9.0],
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0],
    ])
    .unwrap();

    let label = Array2D::from_rows(&[
        &[0u32, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        &[0, 1, 1, 2, 2, 2, 2, 1, 1, 0],
        &[0, 1, 1, 2, 2, 2, 2, 1, 1, 0],
        &[0, 1, 1, 2, 2, 2, 2, 1, 1, 0],
        &[0, 1, 1, 2, 2, 2, 2, 1, 1, 0],
        &[0, 1, 1, 2, 2, 2, 2, 1, 1, 0],
        &[0, 1, 1, 2, 2, 2, 2, 1, 1, 0],
        &[0, 1, 1, 1, 1, 1, 1, 1, 1, 0],
        &[0, 1, 1, 1, 1, 1, 1, 1, 1, 0],
        &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ])
    .unwrap();

    let pit_cell = topo.xy_to_i(4, 2) as FlatIdx;
    let out_cell = topo.xy_to_i(4, 3) as FlatIdx;
    let dep_labels: HashSet<DhLabel> = [2].into_iter().collect();

    // Standard case
    let mut wtd = Array2D::<f64>::new(topo.width(), topo.height(), 0.0);
    fill_depressions_fsm(
        pit_cell,
        out_cell,
        &dep_labels,
        3.0,
        &topo,
        &label,
        &mut wtd,
    )
    .unwrap();
    assert!((wtd.get(4, 2) - 1.5).abs() < 1e-6);
    assert!((wtd.get(5, 2) - 1.5).abs() < 1e-6);

    // Sill absorbs some water
    let mut wtd = Array2D::<f64>::new(topo.width(), topo.height(), 0.0);
    wtd.set(4, 3, -1.0);
    fill_depressions_fsm(
        pit_cell,
        out_cell,
        &dep_labels,
        5.0,
        &topo,
        &label,
        &mut wtd,
    )
    .unwrap();
    assert!((wtd.get(4, 2) - 2.0).abs() < 1e-6);
    assert!((wtd.get(5, 2) - 2.0).abs() < 1e-6);
    assert!((wtd.get(4, 3) - 0.0).abs() < 1e-6);

    // Passes over a saddle and sill absorbs some
    let mut wtd = Array2D::<f64>::new(topo.width(), topo.height(), 0.0);
    let saddle_out = topo.xy_to_i(6, 4) as FlatIdx;
    wtd.set(6, 4, -1.0);
    fill_depressions_fsm(
        pit_cell,
        saddle_out,
        &dep_labels,
        19.5,
        &topo,
        &label,
        &mut wtd,
    )
    .unwrap();
    assert!((wtd.get(6, 4) - (-0.5)).abs() < 1e-6);
    assert!((wtd.get(4, 2) - 3.0).abs() < 1e-6);
    assert!((wtd.get(4, 3) - 1.0).abs() < 1e-6);
}

#[test]
fn test_pq_issues() {
    // PQ Issue 1
    let topo = Array2D::from_rows(&[
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, 9.0, -9.0],
        &[-9.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, -9.0],
        &[-9.0, 5.0, 1.0, 1.0, 5.0, 1.0, 1.0, 5.0, -9.0],
        &[-9.0, 5.0, 1.0, 1.0, 5.0, 1.0, 1.0, 5.0, -9.0],
        &[-9.0, 2.0, 1.0, 1.0, 3.0, 1.0, 1.0, 5.0, -9.0],
        &[-9.0, 5.0, 1.0, 1.0, 5.0, 1.0, 1.0, 5.0, -9.0],
        &[-9.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, -9.0],
        &[-9.0, -9.0, -9.0, -9.0, -9.0, -9.0, -9.0, 9.0, -9.0],
    ])
    .unwrap();
    let mut labels = Array2D::<DhLabel>::new(topo.width(), topo.height(), NO_DEP);
    labels.set_edges(OCEAN);
    let mut flowdirs = Array2D::<Flowdir>::new(topo.width(), topo.height(), NO_FLOW);
    let mut deps =
        get_depression_hierarchy(&topo, &mut labels, &mut flowdirs, Topology::D8).unwrap();
    let mut wtd = Array2D::<f64>::new(topo.width(), topo.height(), 0.0);
    wtd.set(5, 5, 17.0);
    fill_spill_merge(&topo, &labels, &flowdirs, &mut deps, &mut wtd).unwrap();

    // PQ Issue 2
    let topo2 = Array2D::from_rows(&[
        &[-9.0, -9.0, -9.0, -9.0, -9.0],
        &[-9.0, 0.0, 0.0, 0.0, -9.0],
        &[-9.0, 0.0, -8.0, 0.0, -9.0],
        &[-9.0, 0.0, -8.0, 0.0, -9.0],
        &[-9.0, 0.0, 0.0, 0.0, -9.0],
        &[-9.0, -9.0, -9.0, -9.0, -9.0],
    ])
    .unwrap();
    let labels2 = Array2D::from_rows(&[
        &[0u32, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0],
        &[0, 0, 1, 0, 0],
        &[0, 0, 1, 0, 0],
        &[0, 0, 0, 0, 0],
        &[0, 0, 0, 0, 0],
    ])
    .unwrap();
    let dep_labels: HashSet<DhLabel> = [1].into_iter().collect();
    let mut wtd2 = Array2D::<f64>::new(topo2.width(), topo2.height(), 0.0);
    fill_depressions_fsm(
        topo2.xy_to_i(2, 2) as FlatIdx,
        topo2.xy_to_i(3, 3) as FlatIdx,
        &dep_labels,
        2.0,
        &topo2,
        &labels2,
        &mut wtd2,
    )
    .unwrap();
}

#[test]
fn test_randomized_heavy_flooding_vs_priority_flood() {
    let mut rng = StdRng::seed_from_u64(12345);
    for _ in 0..100 {
        let mut dem = random_terrain(&mut rng, 10, 30);
        let mut labels = Array2D::<DhLabel>::new(dem.width(), dem.height(), NO_DEP);
        let mut flowdirs = Array2D::<Flowdir>::new(dem.width(), dem.height(), NO_FLOW);
        dem.set_edges(-1.0);
        labels.set_edges(OCEAN);

        let mut deps =
            get_depression_hierarchy(&dem, &mut labels, &mut flowdirs, Topology::D8).unwrap();
        let mut wtd = Array2D::<f64>::new(dem.width(), dem.height(), 100.0);
        fill_spill_merge(&dem, &labels, &flowdirs, &mut deps, &mut wtd).unwrap();

        let mut comparison = dem.clone();
        priority_flood_zhou2016(&mut comparison);

        for i in 0..dem.size() {
            if !dem.is_no_data_i(i) {
                let v = dem.get_i(i) + wtd.get_i(i);
                dem.set_i(i, v);
            }
        }
        assert!(max_array_diff(&comparison, &dem) < 1e-6);
    }
}

#[test]
fn test_randomized_repeated_fsm_and_incremental_and_mass_conservation() {
    let mut rng = StdRng::seed_from_u64(67890);
    for _ in 0..80 {
        let mut dem = random_integer_terrain(&mut rng, 10, 25);
        dem.set_edges(-1.0);

        // 1. Repeated FSM idempotence
        let mut label = Array2D::<DhLabel>::new(dem.width(), dem.height(), NO_DEP);
        label.set_edges(OCEAN);
        let mut flowdirs = Array2D::<Flowdir>::new(dem.width(), dem.height(), NO_FLOW);
        let mut dh =
            get_depression_hierarchy(&dem, &mut label, &mut flowdirs, Topology::D8).unwrap();
        let mut wtd = Array2D::<f64>::new(dem.width(), dem.height(), 1.0);
        fill_spill_merge(&dem, &label, &flowdirs, &mut dh, &mut wtd).unwrap();
        let first_wtd = wtd.clone();
        fill_spill_merge(&dem, &label, &flowdirs, &mut dh, &mut wtd).unwrap();
        assert!(max_array_diff(&first_wtd, &wtd) < 1e-6);

        // 2. Incremental vs big dump
        let mut wtd_inc = Array2D::<f64>::new(dem.width(), dem.height(), 0.0);
        for _ in 0..10 {
            for v in wtd_inc.data_mut().iter_mut() {
                *v += 0.1;
            }
            fill_spill_merge(&dem, &label, &flowdirs, &mut dh, &mut wtd_inc).unwrap();
        }
        assert!(max_array_diff(&first_wtd, &wtd_inc) < 1e-6);

        // 3. Mass conservation
        let surface_water_amount: f64 = rng.gen_range(0.0..1.0);
        let mut wtd_mc = Array2D::<f64>::new(dem.width(), dem.height(), surface_water_amount);
        fill_spill_merge(&dem, &label, &flowdirs, &mut dh, &mut wtd_mc).unwrap();
        let total: f64 = wtd_mc.data().iter().sum::<f64>() + dh[OCEAN as usize].water_vol;
        let expected = surface_water_amount * (wtd_mc.size() as f64);
        assert!(
            (total - expected).abs() < 1e-4,
            "mass conservation failed: total={total}, expected={expected}"
        );
    }
}
