"""Command-line entry points mirroring RichDEM's `rd_*` Python scripts."""

from __future__ import annotations

import argparse
import sys
from argparse import RawTextHelpFormatter

import numpy as np

import richdem_rs as rd


def DepressionFilling() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=RawTextHelpFormatter, description="RichDEM Depression Filling"
    )
    parser.add_argument("dem", type=str, help="Elevation model")
    parser.add_argument("outname", type=str, help="Name of output file")
    parser.add_argument(
        "-g", "--gradient", action="store_true", help="Impose epsilon gradient"
    )
    parser.add_argument(
        "-v", "--version", action="version", version=rd._RichDEMVersion()
    )
    args = parser.parse_args()

    dem = rd.LoadGDAL(args.dem)
    rd._AddAnalysis(dem, " ".join(sys.argv))
    rd.FillDepressions(dem, epsilon=args.gradient, in_place=True)
    rd.SaveGDAL(args.outname, dem)


def BreachDepressions() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=RawTextHelpFormatter, description="RichDEM Depression Breaching"
    )
    parser.add_argument("dem", type=str, help="Elevation model")
    parser.add_argument("outname", type=str, help="Name of output file")
    parser.add_argument(
        "-v", "--version", action="version", version=rd._RichDEMVersion()
    )
    args = parser.parse_args()

    dem = rd.LoadGDAL(args.dem)
    rd._AddAnalysis(dem, " ".join(sys.argv))
    rd.BreachDepressions(dem, in_place=True)
    rd.SaveGDAL(args.outname, dem)


def FlowAccumulation() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=RawTextHelpFormatter, description="RichDEM Flow Accumulation"
    )
    parser.add_argument("dem", type=str, help="Elevation model")
    parser.add_argument("outname", type=str, help="Name of output file")
    parser.add_argument(
        "-m", "--method", type=str, required=True, help="Flow accumulation method"
    )
    parser.add_argument(
        "-e", "--exponent", type=float, help="Optional exponent (Holmgren/Freeman)"
    )
    parser.add_argument(
        "-v", "--version", action="version", version=rd._RichDEMVersion()
    )
    args = parser.parse_args()

    dem = rd.LoadGDAL(args.dem)
    rd._AddAnalysis(dem, " ".join(sys.argv))
    accum = rd.FlowAccumulation(dem, method=args.method, exponent=args.exponent)
    rd.SaveGDAL(args.outname, accum)


def TerrainAttribute() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=RawTextHelpFormatter, description="RichDEM Terrain Attribute"
    )
    parser.add_argument("dem", type=str, help="Elevation model")
    parser.add_argument("outname", type=str, help="Name of output file")
    parser.add_argument(
        "-a", "--attrib", type=str, required=True, help="Terrain attribute"
    )
    parser.add_argument(
        "-z", "--zscale", type=float, default=1.0, help="Z-scale factor"
    )
    parser.add_argument(
        "-v", "--version", action="version", version=rd._RichDEMVersion()
    )
    args = parser.parse_args()

    dem = rd.LoadGDAL(args.dem)
    rd._AddAnalysis(dem, " ".join(sys.argv))
    tattrib = rd.TerrainAttribute(dem, attrib=args.attrib, zscale=args.zscale)
    rd.SaveGDAL(args.outname, tattrib)


def RdInfo() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=RawTextHelpFormatter, description="RichDEM Dataset Information"
    )
    parser.add_argument("rda", type=str, help="Elevation model")
    args = parser.parse_args()

    rda = rd.LoadGDAL(args.rda)
    print(f"File      = {args.rda}")
    print(f"Data type = {rda.dtype}")
    print(f"Width     = {rda.shape[1]}")
    print(f"Height    = {rda.shape[0]}")
    print(f"Shape     = {rda.shape[1]}x{rda.shape[0]}")
    print("Metadata:")
    for k, v in rda.metadata.items():
        if k != "PROCESSING_HISTORY":
            print(f"\t{k} = {v}")
    print("Processing History:")
    print("-------------------")
    if "PROCESSING_HISTORY" in rda.metadata:
        for ph in rda.metadata["PROCESSING_HISTORY"].split("\n"):
            if ph.strip():
                print(ph.strip())
    print("-------------------")


def RdCompare() -> None:
    parser = argparse.ArgumentParser(
        formatter_class=RawTextHelpFormatter, description="RichDEM Dataset Comparison"
    )
    parser.add_argument("rda1", type=str, help="First raster")
    parser.add_argument("rda2", type=str, help="Second raster")
    args = parser.parse_args()

    ds1 = rd.LoadGDAL(args.rda1)
    ds2 = rd.LoadGDAL(args.rda2)
    diff = np.asarray(ds1, dtype=np.float64) - np.asarray(ds2, dtype=np.float64)
    print(f"Absolute Max difference: {np.nanmax(np.abs(diff)):10.6}")
    print(f"Absolute Min difference: {np.nanmin(np.abs(diff)):10.6}")
    print(f"Absolute Avg difference: {np.mean(np.abs(diff)):10.6}")
    print(f"RMS difference:          {np.sqrt(np.mean(np.square(diff))):10.6}")
