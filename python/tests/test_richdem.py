"""Python test suite verifying parity with pyRichDEM."""

import tempfile
from pathlib import Path

import numpy as np
import pytest
import richdem_rs as rd
from richdem_rs import depression_hierarchy as dephier


def test_depression_defaults() -> None:
    d = dephier.Depression()
    assert d.out_cell == dephier.NO_VALUE
    assert d.pit_cell == dephier.NO_VALUE
    assert d.parent == dephier.NO_PARENT
    assert d.odep == dephier.NO_VALUE
    assert d.geolink == dephier.NO_VALUE
    assert d.lchild == dephier.NO_VALUE
    assert d.rchild == dephier.NO_VALUE
    assert d.ocean_parent is False
    assert d.dep_label == 0
    assert d.cell_count == 0
    assert d.dep_vol == 0
    assert d.water_vol == 0
    assert d.total_elevation == 0


def test_depression_hierarchy_and_fsm() -> None:
    dem = rd.generate_perlin_terrain(20, 20)
    dem[0, :] = -1.0
    dem[-1, :] = -1.0
    dem[:, 0] = -1.0
    dem[:, -1] = -1.0
    labels = rd.get_new_depression_hierarchy_labels(dem.shape)
    dh, flowdirs = rd.get_depression_hierarchy(dem, labels)
    assert len(dh) >= 1
    assert flowdirs.shape == dem.shape

    wtd = rd.rdarray(np.ones(dem.shape, dtype=np.float64), no_data=-9999)
    rd.fill_spill_merge(dem, labels, flowdirs, dh, wtd)
    total = float(np.sum(wtd)) + dh[dephier.OCEAN].water_vol
    assert abs(total - dem.size) < 1e-4


def test_core_hydrology_and_geotiff_io() -> None:
    dem = rd.generate_perlin_terrain(24, 42)
    filled = rd.FillDepressions(dem, epsilon=True)
    assert filled is not None
    assert filled.shape == dem.shape

    breached = rd.BreachDepressions(dem)
    assert breached is not None
    assert breached.shape == dem.shape

    accum_d8 = rd.FlowAccumulation(filled, method="D8")
    assert float(np.max(accum_d8)) >= 1.0

    props = rd.FlowProportions(filled, method="Dinf")
    assert props.shape == (24, 24, 9)
    accum_props = rd.FlowAccumFromProps(props)
    assert float(np.max(accum_props)) >= 1.0

    slope = rd.TerrainAttribute(filled, attrib="slope_degrees")
    assert slope.shape == dem.shape

    with tempfile.TemporaryDirectory() as tmpdir:
        tif = Path(tmpdir) / "dem.tif"
        rd.SaveGDAL(str(tif), filled)
        loaded = rd.LoadGDAL(str(tif))
        assert loaded.shape == filled.shape
        assert np.allclose(np.asarray(loaded), np.asarray(filled))


def test_python_review_regression_cases() -> None:
    # 1. u64 > 2**53 preserved across SaveGDAL / LoadGDAL
    big_val = np.uint64(9007199254740993)
    arr_u64 = rd.rdarray(
        np.array([[big_val, big_val + np.uint64(2)]], dtype=np.uint64),
        no_data=int(np.iinfo(np.uint64).max),
    )
    with tempfile.TemporaryDirectory() as tmpdir:
        tif = Path(tmpdir) / "u64.tif"
        rd.SaveGDAL(str(tif), arr_u64)
        loaded = rd.LoadGDAL(str(tif))
        assert loaded.dtype == np.uint64
        assert int(loaded[0, 0]) == int(big_val)
        assert int(loaded[0, 1]) == int(big_val) + 2

    # 2. float32 epsilon filling preserves non-flat gradient
    f32_dem = rd.rdarray(
        np.array([[1, 1, 1], [1, 0, 1], [1, 1, 1]], dtype=np.float32),
        no_data=-9999.0,
    )
    f32_filled = rd.FillDepressions(f32_dem, epsilon=True)
    assert f32_filled is not None
    assert f32_filled.dtype == np.float32
    assert float(f32_filled[1, 1]) > 1.0

    # 3. Integer weights rejected for in_place accumulation
    dem = rd.generate_perlin_terrain(8, 7)
    filled = rd.FillDepressions(dem, epsilon=True)
    assert filled is not None
    int_weights = rd.rdarray(np.ones(dem.shape, dtype=np.int32), no_data=-9999)
    with pytest.raises(TypeError):
        rd.FlowAccumulation(filled, method="Dinf", weights=int_weights, in_place=True)

    # 4. ArcGIS & TauDEM conversions preserve NoData=255
    arc = rd.rdarray(np.array([[1, 16, 255]], dtype=np.uint8), no_data=255)
    rd_d8 = rd.convert_arc_flowdirs_to_richdem_d8(arc)
    assert int(rd_d8.no_data) == 255
    assert list(rd_d8[0]) == [5, 1, 255]

    taudem = rd.rdarray(np.array([[1, 5, 255]], dtype=np.uint8), no_data=255)
    rd_td = rd.convert_taudem_d8_to_richdem_d8(taudem)
    assert int(rd_td.no_data) == 255
    assert list(rd_td[0]) == [5, 1, 255]

    # Signed int32 TauDEM with no_data=-9999 normalizes cleanly, and invalid/fractional codes fail
    taudem_signed = rd.rdarray(np.array([[1, 5, -9999]], dtype=np.int32), no_data=-9999)
    rd_td_signed = rd.convert_taudem_d8_to_richdem_d8(taudem_signed)
    assert int(rd_td_signed.no_data) == 255
    assert list(rd_td_signed[0]) == [5, 1, 255]

    with pytest.raises(ValueError):
        rd.convert_taudem_d8_to_richdem_d8(
            rd.rdarray(np.array([[1.5]], dtype=np.float64), no_data=255)
        )
    with pytest.raises(ValueError):
        rd.convert_arc_flowdirs_to_richdem_d8(
            rd.rdarray(np.array([[300]], dtype=np.int32), no_data=-9999)
        )

    # 4b. Big-endian arrays save cleanly via SaveGDAL
    be_arr = rd.rdarray(
        np.array([[1.5, 2.5]], dtype=">f4"),
        no_data=-9999.0,
    )
    with tempfile.TemporaryDirectory() as tmpdir:
        tif = Path(tmpdir) / "be.tif"
        rd.SaveGDAL(str(tif), be_arr)
        loaded_be = rd.LoadGDAL(str(tif))
        assert np.allclose(np.asarray(loaded_be), np.array([[1.5, 2.5]]))

    # 5. Shape mismatch rejected in get_depression_hierarchy
    dem6 = rd.rdarray(np.zeros((6, 6), dtype=np.float64), no_data=-9999.0)
    bad_labels = rd.get_new_depression_hierarchy_labels((3, 12))
    with pytest.raises(ValueError):
        rd.get_depression_hierarchy(dem6, bad_labels)

    # 6. NumPy ndarray geotransform works in get_depression_hierarchy
    dem6.geotransform = rd.STANDARD_GEOTRANSFORM.copy()
    ok_labels = rd.get_new_depression_hierarchy_labels((6, 6))
    dh, fds = rd.get_depression_hierarchy(dem6, ok_labels)
    assert len(dh) >= 1
    assert fds.shape == (6, 6)

    # 7. Overflowing Perlin terrain size raises ValueError instead of panicking
    with pytest.raises((ValueError, OverflowError)):
        rd.generate_perlin_terrain(2**40, 42)
