//! Conversions between ArcGIS, TauDEM, and RichDEM D8 flow direction encodings.

use crate::array2d::Array2D;
use crate::constants::{D8Flowdir, FLOWDIR_NO_DATA};
use crate::error::{Error, Result};
use crate::num::DemNum;

/// Converts ArcGIS D8 bit-flag flow directions (`0, 1, 2, 4, 8, 16, 32, 64, 128`)
/// into RichDEM D8 flow directions (`0..=8`), preserving NoData cells with a sentinel
/// outside `0..=8`.
pub fn convert_arc_flowdirs_to_richdem_d8(
    arc_flowdirs: &Array2D<D8Flowdir>,
) -> Result<Array2D<D8Flowdir>> {
    let in_nd = arc_flowdirs.no_data();
    let out_nd = if in_nd > 8 { in_nd } else { FLOWDIR_NO_DATA };
    let mut rd_flowdirs = Array2D::<D8Flowdir>::make_from_template(arc_flowdirs, 0);
    rd_flowdirs.set_no_data(out_nd);

    for i in 0..arc_flowdirs.size() {
        let v = arc_flowdirs.get_i(i);
        if v == in_nd {
            rd_flowdirs.set_i(i, out_nd);
            continue;
        }
        let mapped = match v {
            0 => 0,
            1 => 5,
            2 => 6,
            4 => 7,
            8 => 8,
            16 => 1,
            32 => 2,
            64 => 3,
            128 => 4,
            other => {
                return Err(Error::InvalidArgument(format!(
                    "Unknown ArcGIS flow direction {other}"
                )));
            }
        };
        rd_flowdirs.set_i(i, mapped);
    }

    Ok(rd_flowdirs)
}

/// Converts TauDEM D8 flow directions (`0..=8`) into RichDEM D8 flow directions (`0..=8`),
/// preserving NoData cells with a sentinel outside `0..=8`.
pub fn convert_taudem_d8_to_richdem_d8<T: DemNum>(
    taudem_flowdirs: &Array2D<T>,
) -> Result<Array2D<T>> {
    const TAUDEM_TO_RICHDEM: [i64; 9] = [0, 5, 4, 3, 2, 1, 8, 7, 6];
    let in_nd = taudem_flowdirs.no_data();
    let in_nd_f = in_nd.to_f64();
    let nd_collides = !in_nd.is_nan()
        && (0.0..=8.0).contains(&in_nd_f)
        && (in_nd_f - in_nd_f.trunc()).abs() == 0.0;
    let out_nd = if nd_collides {
        if T::IS_SIGNED {
            T::from_i64(-1)
        } else {
            T::MAX
        }
    } else {
        in_nd
    };

    let mut out = taudem_flowdirs.clone();
    out.set_no_data(out_nd);
    for i in 0..out.size() {
        if taudem_flowdirs.is_no_data_i(i) {
            out.set_i(i, out_nd);
            continue;
        }
        let v = taudem_flowdirs.get_i(i).to_i64();
        if !(0..=8).contains(&v) {
            return Err(Error::InvalidArgument(format!(
                "Invalid TauDEM flow direction '{v}'"
            )));
        }
        out.set_i(i, T::from_i64(TAUDEM_TO_RICHDEM[v as usize]));
    }
    Ok(out)
}
