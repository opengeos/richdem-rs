"""richdem-rs: High-performance DEM hydrologic and terrain analysis in pure Rust.

Full-parity Python API ported from Richard Barnes's RichDEM
(https://github.com/r-barnes/richdem), using terrio for GDAL-free geospatial raster I/O.
"""

from __future__ import annotations

import copy
import datetime
from collections.abc import Iterable
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from typing import Self

import numpy as np

from . import _richdem_rs
from ._richdem_rs import depression_hierarchy

__version__: str = _richdem_rs.__version__
GDAL_AVAILABLE: bool = (
    True  # Provided natively via pure-Rust terrio (no GDAL install required)
)
NO_FLOW: int = _richdem_rs.NO_FLOW
STANDARD_GEOTRANSFORM: np.ndarray = np.array(
    [0.0, 1.0, 0.0, 0.0, 0.0, -1.0], dtype=np.float64
)


def _RichDEMVersion() -> str:
    return f"richdem-rs (Python {__version__})"


def _AddAnalysis(rda: rdarray | rd3array, analysis: str) -> None:
    if not isinstance(rda, (rdarray, rd3array)):
        raise TypeError("An rdarray or rd3array is required!")
    nowdate = datetime.datetime.now(datetime.timezone.utc).strftime(
        "%Y-%m-%d %H:%M:%S.%f UTC"
    )
    metastr = f"\n{nowdate} | {_RichDEMVersion()} | {analysis}"
    if rda.metadata is None:
        rda.metadata = {}
    if "PROCESSING_HISTORY" not in rda.metadata:
        rda.metadata["PROCESSING_HISTORY"] = ""
    rda.metadata["PROCESSING_HISTORY"] += metastr


def _gt_list(gt: Any) -> list[float]:
    if gt is None:
        return [0.0, 1.0, 0.0, 0.0, 0.0, -1.0]
    arr = [float(x) for x in gt]
    if len(arr) != 6:
        raise ValueError(f"geotransform must have 6 elements, got {len(arr)}")
    return arr


class rdarray(np.ndarray):
    """2D NumPy ndarray subclass carrying RichDEM geospatial metadata."""

    def __new__(
        cls,
        array: Any,
        meta_obj: Any = None,
        no_data: float | None = None,
        dtype: Any = None,
        order: Any = None,
        geotransform: Iterable[float] | None = None,
        **kwargs: Any,
    ) -> Self:
        obj = np.asarray(array, dtype=dtype, order=order).view(cls)

        if meta_obj is not None:
            obj.metadata = copy.deepcopy(getattr(meta_obj, "metadata", {}))
            obj.no_data = copy.deepcopy(getattr(meta_obj, "no_data", None))
            obj.projection = copy.deepcopy(getattr(meta_obj, "projection", ""))
            obj.geotransform = copy.deepcopy(getattr(meta_obj, "geotransform", None))
        elif geotransform is not None:
            obj.geotransform = list(geotransform)
        else:
            obj.metadata = {}
            obj.projection = ""
            obj.geotransform = None

        if no_data is not None:
            obj.no_data = no_data

        if getattr(obj, "no_data", None) is None:
            raise ValueError("A no_data value must be specified!")

        return obj

    def __array_finalize__(self, obj: Any) -> None:
        if obj is None:
            return
        self.metadata = copy.deepcopy(getattr(obj, "metadata", {}))
        self.no_data = copy.deepcopy(getattr(obj, "no_data", None))
        self.projection = copy.deepcopy(getattr(obj, "projection", ""))
        self.geotransform = copy.deepcopy(getattr(obj, "geotransform", None))


class rd3array(np.ndarray):
    """3D (height, width, 9) NumPy ndarray subclass for RichDEM flow proportions."""

    def __new__(
        cls,
        array: Any,
        meta_obj: Any = None,
        no_data: float | None = None,
        order: Any = None,
        **kwargs: Any,
    ) -> Self:
        obj = np.asarray(array, dtype=np.float32, order=order).view(cls)

        if meta_obj is not None:
            obj.metadata = copy.deepcopy(getattr(meta_obj, "metadata", {}))
            obj.no_data = copy.deepcopy(getattr(meta_obj, "no_data", None))
            obj.projection = copy.deepcopy(getattr(meta_obj, "projection", ""))
            obj.geotransform = copy.deepcopy(getattr(meta_obj, "geotransform", None))
        else:
            obj.metadata = {}
            obj.projection = ""
            obj.geotransform = None

        if no_data is not None:
            obj.no_data = no_data

        if getattr(obj, "no_data", None) is None:
            raise ValueError("A no_data value must be specified!")

        return obj

    def __array_finalize__(self, obj: Any) -> None:
        if obj is None:
            return
        self.metadata = copy.deepcopy(getattr(obj, "metadata", {}))
        self.no_data = copy.deepcopy(getattr(obj, "no_data", None))
        self.projection = copy.deepcopy(getattr(obj, "projection", ""))
        self.geotransform = copy.deepcopy(getattr(obj, "geotransform", None))


def LoadGDAL(filename: str, no_data: float | None = None) -> rdarray:
    """Read a geospatial raster into an `rdarray` using pure-Rust `terrio` (no GDAL needed)."""
    arr, nd, gt, proj, meta = _richdem_rs.load_raster_terrio(str(filename))
    effective_nd = no_data if no_data is not None else nd
    rda = rdarray(arr, no_data=effective_nd, geotransform=gt)
    rda.projection = proj
    rda.metadata = dict(meta)
    _AddAnalysis(rda, f"LoadGDAL(filename={filename}, no_data={no_data})")
    return rda


def SaveGDAL(filename: str, rda: rdarray, compress: bool = False) -> None:
    """Save an `rdarray` to GeoTIFF/COG/ASCII/etc. using pure-Rust `terrio`."""
    if not isinstance(rda, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    supported_dtypes = (
        np.uint8,
        np.int8,
        np.uint16,
        np.int16,
        np.uint32,
        np.int32,
        np.uint64,
        np.int64,
        np.float32,
        np.float64,
    )
    if rda.dtype.type in supported_dtypes:
        data = np.ascontiguousarray(rda)
        nd = (
            int(rda.no_data)
            if np.issubdtype(rda.dtype, np.integer)
            else float(rda.no_data)
        )
    else:
        data = np.ascontiguousarray(rda, dtype=np.float64)
        nd = float(rda.no_data)
    meta = {str(k): str(v) for k, v in (rda.metadata or {}).items()}
    _richdem_rs.save_raster_terrio(
        str(filename),
        data,
        nd,
        _gt_list(rda.geotransform),
        str(rda.projection or ""),
        meta,
        bool(compress),
    )


def FillDepressions(
    dem: rdarray,
    epsilon: bool = False,
    in_place: bool = False,
    topology: str = "D8",
) -> rdarray | None:
    """Fills all depressions in a DEM."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    if epsilon and not np.issubdtype(dem.dtype, np.floating):
        raise TypeError(
            "FillDepressions(epsilon=True) requires a floating-point DEM dtype"
        )
    if epsilon and dem.dtype == np.float32:
        data = np.ascontiguousarray(dem, dtype=np.float32)
        filled = _richdem_rs.fill_depressions_f32(
            data, float(dem.no_data), True, topology
        )
    else:
        data = np.ascontiguousarray(dem, dtype=np.float64)
        filled = _richdem_rs.fill_depressions_f64(
            data, float(dem.no_data), bool(epsilon), topology
        )
    if in_place:
        dem[...] = filled.astype(dem.dtype, copy=False)
        _AddAnalysis(dem, f"FillDepressions(dem, epsilon={epsilon})")
        return None
    out = rdarray(
        filled.astype(dem.dtype, copy=False), meta_obj=dem, no_data=dem.no_data
    )
    _AddAnalysis(out, f"FillDepressions(dem, epsilon={epsilon})")
    return out


def BreachDepressions(
    dem: rdarray,
    in_place: bool = False,
    topology: str = "D8",
) -> rdarray | None:
    """Breaches all depressions in a DEM using Lindsay (2016)."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    data = np.ascontiguousarray(dem, dtype=np.float64)
    breached = _richdem_rs.breach_depressions_f64(data, float(dem.no_data), topology)
    if in_place:
        dem[...] = breached.astype(dem.dtype, copy=False)
        _AddAnalysis(dem, "BreachDepressions(dem)")
        return None
    out = rdarray(
        breached.astype(dem.dtype, copy=False), meta_obj=dem, no_data=dem.no_data
    )
    _AddAnalysis(out, "BreachDepressions(dem)")
    return out


def ResolveFlats(dem: rdarray, in_place: bool = False) -> rdarray | None:
    """Resolves flat surfaces by imposing a local epsilon gradient (Barnes 2014)."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    if not np.issubdtype(dem.dtype, np.floating):
        raise TypeError("ResolveFlats requires a floating-point DEM dtype")
    if dem.dtype == np.float32:
        data = np.ascontiguousarray(dem, dtype=np.float32)
        resolved = _richdem_rs.resolve_flats_f32(data, float(dem.no_data))
    else:
        data = np.ascontiguousarray(dem, dtype=np.float64)
        resolved = _richdem_rs.resolve_flats_f64(data, float(dem.no_data))
    if in_place:
        dem[...] = resolved.astype(dem.dtype, copy=False)
        _AddAnalysis(dem, f"ResolveFlats(dem, in_place={in_place})")
        return None
    out = rdarray(
        resolved.astype(dem.dtype, copy=False), meta_obj=dem, no_data=dem.no_data
    )
    _AddAnalysis(out, f"ResolveFlats(dem, in_place={in_place})")
    return out


def FlowProportions(
    dem: rdarray,
    method: str,
    exponent: float | None = None,
) -> rd3array:
    """Calculates 3D `(H, W, 9)` flow proportions for `method`."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    data = np.ascontiguousarray(dem, dtype=np.float64)
    props = _richdem_rs.flow_proportions_f64(data, float(dem.no_data), method, exponent)
    out = rd3array(props, meta_obj=dem, no_data=-2)
    _AddAnalysis(out, f"FlowProportions(dem, method={method}, exponent={exponent})")
    return out


def FlowAccumFromProps(
    props: rd3array,
    weights: rdarray | None = None,
    in_place: bool = False,
) -> rdarray:
    """Calculates flow accumulation from 3D flow proportions."""
    if not isinstance(props, rd3array):
        raise TypeError("A richdem_rs.rd3array is required!")
    if (
        weights is not None
        and in_place
        and not np.issubdtype(weights.dtype, np.floating)
    ):
        raise TypeError(
            "FlowAccumFromProps(in_place=True) requires floating-point weights"
        )
    w_arr = (
        np.ascontiguousarray(weights, dtype=np.float64) if weights is not None else None
    )
    accum_np = _richdem_rs.flow_accum_from_props(
        np.ascontiguousarray(props, dtype=np.float32), w_arr
    )
    if weights is not None and in_place:
        weights[...] = accum_np
        weights.no_data = -1
        _AddAnalysis(
            weights, f"FlowAccumFromProps(props, weights=weights, in_place={in_place})"
        )
        return weights
    out = rdarray(accum_np, meta_obj=props, no_data=-1)
    _AddAnalysis(
        out,
        f"FlowAccumFromProps(props, weights={'weights' if weights is not None else 'None'}, in_place={in_place})",
    )
    return out


def FlowAccumulation(
    dem: rdarray,
    method: str,
    exponent: float | None = None,
    weights: rdarray | None = None,
    in_place: bool = False,
) -> rdarray:
    """Calculates flow accumulation using any RichDEM flow routing method."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    if (
        weights is not None
        and in_place
        and not np.issubdtype(weights.dtype, np.floating)
    ):
        raise TypeError(
            "FlowAccumulation(in_place=True) requires floating-point weights"
        )
    data = np.ascontiguousarray(dem, dtype=np.float64)
    w_arr = (
        np.ascontiguousarray(weights, dtype=np.float64) if weights is not None else None
    )
    accum_np = _richdem_rs.flow_accumulation_f64(
        data, float(dem.no_data), method, exponent, w_arr
    )
    if weights is not None and in_place:
        weights[...] = accum_np
        weights.no_data = -1
        _AddAnalysis(
            weights,
            f"FlowAccumulation(dem, method={method}, exponent={exponent}, in_place=True)",
        )
        return weights
    out = rdarray(accum_np, meta_obj=dem, no_data=-1)
    _AddAnalysis(
        out,
        f"FlowAccumulation(dem, method={method}, exponent={exponent}, in_place={in_place})",
    )
    return out


def TerrainAttribute(dem: rdarray, attrib: str, zscale: float = 1.0) -> rdarray:
    """Calculates a terrain attribute (slope, aspect, or curvature)."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    data = np.ascontiguousarray(dem, dtype=np.float64)
    res = _richdem_rs.terrain_attribute_f64(
        data, float(dem.no_data), _gt_list(dem.geotransform), attrib, float(zscale)
    )
    out = rdarray(res, meta_obj=dem, no_data=-9999)
    _AddAnalysis(out, f"TerrainAttribute(dem, attrib={attrib}, zscale={zscale})")
    return out


def generate_perlin_terrain(size: int, seed: int) -> rdarray:
    """Generates a square synthetic Perlin-noise terrain DEM."""
    arr = _richdem_rs.generate_perlin_terrain_py(int(size), int(seed))
    dem = rdarray(arr, no_data=-9999, geotransform=STANDARD_GEOTRANSFORM)
    _AddAnalysis(dem, f"GeneratePerlinTerrain(size={size}, seed={seed})")
    return dem


def get_new_depression_hierarchy_labels(
    shape: tuple[int, int],
    no_data: float = -9999,
    geotransform: np.ndarray | None = None,
) -> rdarray:
    """Creates a labels array with OCEAN on borders and NO_DEP in the interior."""
    gt = geotransform if geotransform is not None else STANDARD_GEOTRANSFORM
    arr = np.full(shape, depression_hierarchy.OCEAN, dtype=np.uint32)
    arr[1:-1, 1:-1] = depression_hierarchy.NO_DEP
    return rdarray(arr, no_data=no_data, geotransform=gt)


def get_depression_hierarchy(
    dem: rdarray,
    labels: rdarray,
    topology: str = "D8",
) -> tuple[list[depression_hierarchy.Depression], rdarray]:
    """Computes the Depression Hierarchy and downhill flow directions."""
    if not isinstance(dem, rdarray):
        raise TypeError("A richdem_rs.rdarray is required!")
    dem_f64 = np.ascontiguousarray(dem, dtype=np.float64)
    if labels.dtype != np.uint32 or not labels.flags["C_CONTIGUOUS"]:
        raise TypeError("labels must be a C-contiguous uint32 rdarray")
    dh, fds = depression_hierarchy.get_depression_hierarchy_py(
        dem_f64, labels, topology
    )
    flowdirs = rdarray(
        fds,
        no_data=-9999,
        geotransform=(
            dem.geotransform if dem.geotransform is not None else STANDARD_GEOTRANSFORM
        ),
    )
    return dh, flowdirs


def fill_spill_merge(
    dem: rdarray,
    labels: rdarray,
    flowdirs: rdarray,
    deps: list[depression_hierarchy.Depression],
    wtd: rdarray,
) -> None:
    """Runs Fill-Spill-Merge, updating `deps` and `wtd` in place."""
    if not all(isinstance(x, rdarray) for x in (dem, labels, flowdirs, wtd)):
        raise TypeError("dem, labels, flowdirs, and wtd must all be rdarray instances!")
    if wtd.dtype != np.float64 or not wtd.flags["C_CONTIGUOUS"]:
        raise TypeError("wtd must be a C-contiguous float64 rdarray")
    depression_hierarchy.fill_spill_merge_py(
        np.ascontiguousarray(dem, dtype=np.float64),
        np.ascontiguousarray(labels, dtype=np.uint32),
        np.ascontiguousarray(flowdirs, dtype=np.int8),
        deps,
        wtd,
    )


def flow_accumulation_from_d8(d8: rdarray) -> rdarray:
    """Computes interior-cell flow accumulation from a RichDEM D8 flow direction raster."""
    arr = np.ascontiguousarray(d8, dtype=np.int32)
    nd = int(d8.no_data) if isinstance(d8, rdarray) else -1
    out = _richdem_rs.flow_accumulation_from_d8_py(arr, nd)
    return rdarray(
        out,
        meta_obj=d8 if isinstance(d8, rdarray) else None,
        no_data=int(np.iinfo(np.uint32).max),
    )


def convert_arc_flowdirs_to_richdem_d8(arc_flowdirs: rdarray) -> rdarray:
    """Converts ArcGIS D8 flow directions to RichDEM D8 flow directions."""
    arr = np.ascontiguousarray(arc_flowdirs, dtype=np.uint8)
    in_nd = int(arc_flowdirs.no_data) if isinstance(arc_flowdirs, rdarray) else 255
    out, out_nd = _richdem_rs.convert_arc_flowdirs_py(arr, in_nd)
    return rdarray(
        out,
        meta_obj=arc_flowdirs if isinstance(arc_flowdirs, rdarray) else None,
        no_data=int(out_nd),
    )


def rdShow(
    rda: rdarray,
    ignore_colours: Iterable[float] = (),
    show: bool = True,
    axes: bool = True,
    cmap: str = "gray",
    vmin: float | None = None,
    vmax: float | None = None,
    figsize: tuple[int, int] = (4, 4),
) -> dict[str, float]:
    """Displays an `rdarray` using matplotlib."""
    import matplotlib.pyplot as plt

    disparr = np.array(rda, dtype=np.float64, copy=True)
    disparr[disparr == rda.no_data] = np.nan
    for c in ignore_colours:
        disparr[disparr == c] = np.nan
    vmin_calc, vmax_calc = np.nanpercentile(disparr, [2, 98])
    vmin_val = float(vmin_calc if vmin is None else vmin)
    vmax_val = float(vmax_calc if vmax is None else vmax)

    fig, (ax, cax) = plt.subplots(
        ncols=2, figsize=figsize, gridspec_kw={"width_ratios": [1, 0.05]}
    )
    iax = ax.imshow(disparr, vmin=vmin_val, vmax=vmax_val, cmap=cmap)
    fig.colorbar(iax, cax=cax)
    plt.tight_layout()
    if not axes:
        ax.axis("off")
    if show:
        plt.show()
    return {"vmin": vmin_val, "vmax": vmax_val}


__all__ = (
    "GDAL_AVAILABLE",
    "NO_FLOW",
    "STANDARD_GEOTRANSFORM",
    "BreachDepressions",
    "FillDepressions",
    "FlowAccumFromProps",
    "FlowAccumulation",
    "FlowProportions",
    "LoadGDAL",
    "ResolveFlats",
    "SaveGDAL",
    "TerrainAttribute",
    "__version__",
    "convert_arc_flowdirs_to_richdem_d8",
    "depression_hierarchy",
    "fill_spill_merge",
    "flow_accumulation_from_d8",
    "generate_perlin_terrain",
    "get_depression_hierarchy",
    "get_new_depression_hierarchy_labels",
    "rd3array",
    "rdShow",
    "rdarray",
)
