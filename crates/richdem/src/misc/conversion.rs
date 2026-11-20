//! Conversions between ArcGIS, TauDEM, and RichDEM D8 flow direction encodings.

use crate::array2d::Array2D;
use crate::constants::D8Flowdir;
use crate::error::{Error, Result};
use crate::num::DemNum;

/// Converts ArcGIS D8 bit-flag flow directions (`0, 1, 2, 4, 8, 16, 32, 64, 128`)
/// into RichDEM D8 flow directions (`0..=8`).
pub fn convert_arc_flowdirs_to_richdem_d8(
    arc_flowdirs: &Array2D<D8Flowdir>,
) -> Result<Array2D<D8Flowdir>> {
    let mut rd_flowdirs = Array2D::<D8Flowdir>::make_from_template(arc_flowdirs, 0);
    rd_flowdirs.set_no_data(arc_flowdirs.no_data());

    for i in 0..arc_flowdirs.size() {
        let v = arc_flowdirs.get_i(i);
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

/// Converts TauDEM D8 flow directions (`0..=8`) into RichDEM D8 flow directions (`0..=8`).
pub fn convert_taudem_d8_to_richdem_d8<T: DemNum>(
    taudem_flowdirs: &Array2D<T>,
) -> Result<Array2D<T>> {
    const TAUDEM_TO_RICHDEM: [i64; 9] = [0, 5, 4, 3, 2, 1, 8, 7, 6];
    let mut out = taudem_flowdirs.clone();
    for i in 0..out.size() {
        if out.is_no_data_i(i) {
            continue;
        }
        let v = out.get_i(i).to_i64();
        if !(0..=8).contains(&v) {
            return Err(Error::InvalidArgument(format!(
                "Invalid TauDEM flow direction '{v}'"
            )));
        }
        out.set_i(i, T::from_i64(TAUDEM_TO_RICHDEM[v as usize]));
    }
    Ok(out)
}
