"""Python test suite verifying parity with pyRichDEM."""

from pathlib import Path
import tempfile

import numpy as np
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
