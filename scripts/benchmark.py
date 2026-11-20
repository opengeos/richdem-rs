#!/usr/bin/env python3
"""Head-to-head Python benchmark: richdem-rs (Rust/PyO3) vs richdem2 (C++/pybind11).

Run from the python virtualenv where both `richdem2` and `richdem_rs` (`--release`)
are installed:

    python scripts/benchmark.py
"""

from __future__ import annotations

import platform
import statistics
import time

import numpy as np
import richdem as rd_cpp
import richdem_rs as rd_rs


def bench_median(fn, warmup: int = 1, iters: int = 3) -> tuple[float, object]:
    out = None
    for _ in range(warmup):
        out = fn()
    times = []
    for _ in range(iters):
        t0 = time.perf_counter()
        out = fn()
        times.append((time.perf_counter() - t0) * 1e3)
    return statistics.median(times), out


def compare_suite(size: int, iters: int = 3) -> None:
    cells = size * size
    print(
        f"\n=== Python Benchmark: {size} x {size} ({cells:,} cells, float64, median of {iters} runs) ==="
    )
    print(
        f"| {'Operation':<34} | {'richdem2 (C++)':>14} | {'richdem-rs (Rust)':>17} | {'Speedup':>9} | {'Max |diff|':>11} |"
    )
    print(f"|{'-' * 36}|{'-' * 16}|{'-' * 19}|{'-' * 11}|{'-' * 13}|")

    # Generate identical synthetic Perlin DEM for both libraries
    base_arr = np.ascontiguousarray(
        rd_rs.generate_perlin_terrain(size, 42), dtype=np.float64
    )
    gt = [0.0, 1.0, 0.0, 0.0, 0.0, -1.0]

    dem_cpp = rd_cpp.rdarray(base_arr.copy(), no_data=-9999.0)
    dem_cpp.geotransform = gt
    dem_rs = rd_rs.rdarray(base_arr.copy(), no_data=-9999.0, geotransform=gt)

    def run_case(label: str, fn_cpp, fn_rs, check_diff: bool = True):
        t_cpp, out_cpp = bench_median(fn_cpp, warmup=1, iters=iters)
        t_rs, out_rs = bench_median(fn_rs, warmup=1, iters=iters)
        speedup = t_cpp / t_rs
        if check_diff and out_cpp is not None and out_rs is not None:
            a = np.asarray(out_cpp, dtype=np.float64)
            b = np.asarray(out_rs, dtype=np.float64)
            nd_a = getattr(out_cpp, "no_data", None)
            nd_b = getattr(out_rs, "no_data", None)
            a_missing = ~np.isfinite(a) | (
                a == float(nd_a) if nd_a is not None else False
            )
            b_missing = ~np.isfinite(b) | (
                b == float(nd_b) if nd_b is not None else False
            )
            mismatch = int(np.sum(a_missing != b_missing))
            valid = ~a_missing & ~b_missing
            if mismatch > 0:
                diff_str = f"mis={mismatch}"
            elif np.any(valid):
                max_diff = float(np.max(np.abs(a[valid] - b[valid])))
                diff_str = f"{max_diff:.2e}"
            else:
                diff_str = "n/a"
        else:
            diff_str = "n/a"
        print(
            f"| {label:<34} | {t_cpp:>11.1f} ms | {t_rs:>14.1f} ms | {speedup:>8.2f}x | {diff_str:>11} |"
        )
        return out_cpp, out_rs

    filled_cpp, filled_rs = run_case(
        "FillDepressions (Zhou2016)",
        lambda: rd_cpp.FillDepressions(dem_cpp, epsilon=False, in_place=False),
        lambda: rd_rs.FillDepressions(dem_rs, epsilon=False, in_place=False),
    )

    filled_eps_cpp, filled_eps_rs = run_case(
        "FillDepressions (epsilon=True)",
        lambda: rd_cpp.FillDepressions(dem_cpp, epsilon=True, in_place=False),
        lambda: rd_rs.FillDepressions(dem_rs, epsilon=True, in_place=False),
    )

    run_case(
        "BreachDepressions (Lindsay2016)",
        lambda: rd_cpp.BreachDepressions(dem_cpp, in_place=False),
        lambda: rd_rs.BreachDepressions(dem_rs, in_place=False),
    )

    run_case(
        "ResolveFlats (Barnes2014)",
        lambda: rd_cpp.ResolveFlats(filled_cpp, in_place=False),
        lambda: rd_rs.ResolveFlats(filled_rs, in_place=False),
    )

    run_case(
        "FlowAccumulation (D8)",
        lambda: rd_cpp.FlowAccumulation(filled_eps_cpp, method="D8"),
        lambda: rd_rs.FlowAccumulation(filled_eps_rs, method="D8"),
    )

    run_case(
        "FlowAccumulation (Dinf)",
        lambda: rd_cpp.FlowAccumulation(filled_eps_cpp, method="Dinf"),
        lambda: rd_rs.FlowAccumulation(filled_eps_rs, method="Dinf"),
    )

    run_case(
        "FlowAccumulation (Quinn)",
        lambda: rd_cpp.FlowAccumulation(filled_eps_cpp, method="Quinn"),
        lambda: rd_rs.FlowAccumulation(filled_eps_rs, method="Quinn"),
    )

    run_case(
        "TerrainAttribute (slope_degrees)",
        lambda: rd_cpp.TerrainAttribute(filled_eps_cpp, attrib="slope_degrees"),
        lambda: rd_rs.TerrainAttribute(filled_eps_rs, attrib="slope_degrees"),
    )

    run_case(
        "TerrainAttribute (curvature)",
        lambda: rd_cpp.TerrainAttribute(filled_eps_cpp, attrib="curvature"),
        lambda: rd_rs.TerrainAttribute(filled_eps_rs, attrib="curvature"),
    )


def main() -> None:
    print(f"Platform: {platform.platform()}")
    print(f"richdem2 (C++): {rd_cpp.__version__}")
    print(f"richdem-rs (Rust): {rd_rs.__version__}")
    compare_suite(1000, iters=3)
    compare_suite(2000, iters=3)


if __name__ == "__main__":
    main()
