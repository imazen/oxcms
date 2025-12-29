//! LUT Tag Types
//!
//! LUT (Look-Up Table) tags define complex color transformations.
//!
//! Types:
//! - mft1 (Lut8Type): 8-bit precision LUT
//! - mft2 (Lut16Type): 16-bit precision LUT
//! - mAB (lutAToBType): v4 A-to-B transform
//! - mBA (lutBToAType): v4 B-to-A transform
//!
//! See ICC.1:2022 Sections 10.10-10.13

use crate::icc::error::IccError;
use crate::icc::types::S15Fixed16;

/// 8-bit LUT data (mft1 / Lut8Type)
#[derive(Debug, Clone)]
pub struct Lut8Data {
    /// Number of input channels
    pub input_channels: u8,
    /// Number of output channels
    pub output_channels: u8,
    /// Number of CLUT grid points
    pub grid_points: u8,
    /// 3x3 matrix (stored row-major)
    pub matrix: [[S15Fixed16; 3]; 3],
    /// Input curves (one per input channel, 256 entries each)
    pub input_curves: Vec<Vec<u8>>,
    /// CLUT data
    pub clut: Vec<u8>,
    /// Output curves (one per output channel, 256 entries each)
    pub output_curves: Vec<Vec<u8>>,
}

impl Lut8Data {
    /// Parse Lut8 data from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 48 {
            return Err(IccError::CorruptedData("Lut8 tag too small".to_string()));
        }

        let input_channels = data[0];
        let output_channels = data[1];
        let grid_points = data[2];
        // data[3] is padding

        // Parse 3x3 matrix (e00-e22) as s15Fixed16
        let mut matrix = [[S15Fixed16::default(); 3]; 3];
        for row in 0..3 {
            for col in 0..3 {
                let offset = 4 + (row * 3 + col) * 4;
                matrix[row][col] = S15Fixed16::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
            }
        }

        let table_offset = 40; // After header + matrix

        // Input tables: inputChannels * 256 bytes
        let input_table_size = input_channels as usize * 256;
        if data.len() < table_offset + input_table_size {
            return Err(IccError::CorruptedData(
                "Lut8 input tables truncated".to_string(),
            ));
        }

        let mut input_curves = Vec::with_capacity(input_channels as usize);
        for i in 0..input_channels as usize {
            let start = table_offset + i * 256;
            input_curves.push(data[start..start + 256].to_vec());
        }

        // CLUT: gridPoints^inputChannels * outputChannels bytes
        let clut_offset = table_offset + input_table_size;
        let clut_size =
            (grid_points as usize).pow(input_channels as u32) * output_channels as usize;

        if data.len() < clut_offset + clut_size {
            return Err(IccError::CorruptedData("Lut8 CLUT truncated".to_string()));
        }

        let clut = data[clut_offset..clut_offset + clut_size].to_vec();

        // Output tables: outputChannels * 256 bytes
        let output_offset = clut_offset + clut_size;
        let output_table_size = output_channels as usize * 256;

        if data.len() < output_offset + output_table_size {
            return Err(IccError::CorruptedData(
                "Lut8 output tables truncated".to_string(),
            ));
        }

        let mut output_curves = Vec::with_capacity(output_channels as usize);
        for i in 0..output_channels as usize {
            let start = output_offset + i * 256;
            output_curves.push(data[start..start + 256].to_vec());
        }

        Ok(Self {
            input_channels,
            output_channels,
            grid_points,
            matrix,
            input_curves,
            clut,
            output_curves,
        })
    }

    /// Check if the matrix is identity
    pub fn matrix_is_identity(&self) -> bool {
        for row in 0..3 {
            for col in 0..3 {
                let expected = if row == col { 1.0 } else { 0.0 };
                if (self.matrix[row][col].to_f64() - expected).abs() > 1e-6 {
                    return false;
                }
            }
        }
        true
    }
}

/// 16-bit LUT data (mft2 / Lut16Type)
#[derive(Debug, Clone)]
pub struct Lut16Data {
    /// Number of input channels
    pub input_channels: u8,
    /// Number of output channels
    pub output_channels: u8,
    /// Number of CLUT grid points
    pub grid_points: u8,
    /// 3x3 matrix (stored row-major)
    pub matrix: [[S15Fixed16; 3]; 3],
    /// Number of input table entries
    pub input_entries: u16,
    /// Number of output table entries
    pub output_entries: u16,
    /// Input curves (one per input channel)
    pub input_curves: Vec<Vec<u16>>,
    /// CLUT data
    pub clut: Vec<u16>,
    /// Output curves (one per output channel)
    pub output_curves: Vec<Vec<u16>>,
}

impl Lut16Data {
    /// Parse Lut16 data from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 52 {
            return Err(IccError::CorruptedData("Lut16 tag too small".to_string()));
        }

        let input_channels = data[0];
        let output_channels = data[1];
        let grid_points = data[2];
        // data[3] is padding

        // Parse 3x3 matrix
        let mut matrix = [[S15Fixed16::default(); 3]; 3];
        for row in 0..3 {
            for col in 0..3 {
                let offset = 4 + (row * 3 + col) * 4;
                matrix[row][col] = S15Fixed16::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
            }
        }

        let input_entries = u16::from_be_bytes([data[40], data[41]]);
        let output_entries = u16::from_be_bytes([data[42], data[43]]);

        let table_offset = 44; // After header + matrix + entry counts

        // Input tables: inputChannels * inputEntries * 2 bytes
        let input_table_size = input_channels as usize * input_entries as usize * 2;
        if data.len() < table_offset + input_table_size {
            return Err(IccError::CorruptedData(
                "Lut16 input tables truncated".to_string(),
            ));
        }

        let mut input_curves = Vec::with_capacity(input_channels as usize);
        for i in 0..input_channels as usize {
            let mut curve = Vec::with_capacity(input_entries as usize);
            for j in 0..input_entries as usize {
                let offset = table_offset + (i * input_entries as usize + j) * 2;
                curve.push(u16::from_be_bytes([data[offset], data[offset + 1]]));
            }
            input_curves.push(curve);
        }

        // CLUT: gridPoints^inputChannels * outputChannels * 2 bytes
        let clut_offset = table_offset + input_table_size;
        let clut_entries =
            (grid_points as usize).pow(input_channels as u32) * output_channels as usize;
        let clut_size = clut_entries * 2;

        if data.len() < clut_offset + clut_size {
            return Err(IccError::CorruptedData("Lut16 CLUT truncated".to_string()));
        }

        let mut clut = Vec::with_capacity(clut_entries);
        for i in 0..clut_entries {
            let offset = clut_offset + i * 2;
            clut.push(u16::from_be_bytes([data[offset], data[offset + 1]]));
        }

        // Output tables
        let output_offset = clut_offset + clut_size;
        let output_table_size = output_channels as usize * output_entries as usize * 2;

        if data.len() < output_offset + output_table_size {
            return Err(IccError::CorruptedData(
                "Lut16 output tables truncated".to_string(),
            ));
        }

        let mut output_curves = Vec::with_capacity(output_channels as usize);
        for i in 0..output_channels as usize {
            let mut curve = Vec::with_capacity(output_entries as usize);
            for j in 0..output_entries as usize {
                let offset = output_offset + (i * output_entries as usize + j) * 2;
                curve.push(u16::from_be_bytes([data[offset], data[offset + 1]]));
            }
            output_curves.push(curve);
        }

        Ok(Self {
            input_channels,
            output_channels,
            grid_points,
            matrix,
            input_entries,
            output_entries,
            input_curves,
            clut,
            output_curves,
        })
    }
}

/// LUT A to B data (mAB / lutAToBType) - v4 profiles
#[derive(Debug, Clone)]
pub struct LutAToBData {
    /// Number of input channels
    pub input_channels: u8,
    /// Number of output channels
    pub output_channels: u8,
    /// B curves (output side)
    pub b_curves: Option<Vec<CurveSegment>>,
    /// Matrix (optional)
    pub matrix: Option<LutMatrix>,
    /// M curves (after matrix)
    pub m_curves: Option<Vec<CurveSegment>>,
    /// CLUT (optional)
    pub clut: Option<LutClut>,
    /// A curves (input side)
    pub a_curves: Option<Vec<CurveSegment>>,
}

impl LutAToBData {
    /// Parse lutAToB data from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 32 {
            return Err(IccError::CorruptedData("lutAToB tag too small".to_string()));
        }

        let input_channels = data[0];
        let output_channels = data[1];
        // data[2..4] reserved

        // Offsets (0 means not present)
        let b_offset = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let matrix_offset = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let m_offset = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let clut_offset = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let a_offset = u32::from_be_bytes([data[20], data[21], data[22], data[23]]) as usize;

        // Parse optional components
        let b_curves = if b_offset != 0 {
            Some(parse_curve_set(data, b_offset, output_channels as usize)?)
        } else {
            None
        };

        let matrix = if matrix_offset != 0 {
            Some(LutMatrix::parse(&data[matrix_offset..])?)
        } else {
            None
        };

        let m_curves = if m_offset != 0 {
            Some(parse_curve_set(data, m_offset, output_channels as usize)?)
        } else {
            None
        };

        let clut = if clut_offset != 0 {
            Some(LutClut::parse(
                &data[clut_offset..],
                input_channels,
                output_channels,
            )?)
        } else {
            None
        };

        let a_curves = if a_offset != 0 {
            Some(parse_curve_set(data, a_offset, input_channels as usize)?)
        } else {
            None
        };

        Ok(Self {
            input_channels,
            output_channels,
            b_curves,
            matrix,
            m_curves,
            clut,
            a_curves,
        })
    }
}

/// LUT B to A data (mBA / lutBToAType) - v4 profiles
#[derive(Debug, Clone)]
pub struct LutBToAData {
    /// Number of input channels
    pub input_channels: u8,
    /// Number of output channels
    pub output_channels: u8,
    /// B curves (input side)
    pub b_curves: Option<Vec<CurveSegment>>,
    /// Matrix (optional)
    pub matrix: Option<LutMatrix>,
    /// M curves (after matrix)
    pub m_curves: Option<Vec<CurveSegment>>,
    /// CLUT (optional)
    pub clut: Option<LutClut>,
    /// A curves (output side)
    pub a_curves: Option<Vec<CurveSegment>>,
}

impl LutBToAData {
    /// Parse lutBToA data from bytes (after type signature and reserved bytes)
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 32 {
            return Err(IccError::CorruptedData("lutBToA tag too small".to_string()));
        }

        let input_channels = data[0];
        let output_channels = data[1];

        // Offsets (0 means not present)
        let b_offset = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let matrix_offset = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let m_offset = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
        let clut_offset = u32::from_be_bytes([data[16], data[17], data[18], data[19]]) as usize;
        let a_offset = u32::from_be_bytes([data[20], data[21], data[22], data[23]]) as usize;

        // Parse optional components
        let b_curves = if b_offset != 0 {
            Some(parse_curve_set(data, b_offset, input_channels as usize)?)
        } else {
            None
        };

        let matrix = if matrix_offset != 0 {
            Some(LutMatrix::parse(&data[matrix_offset..])?)
        } else {
            None
        };

        let m_curves = if m_offset != 0 {
            Some(parse_curve_set(data, m_offset, input_channels as usize)?)
        } else {
            None
        };

        let clut = if clut_offset != 0 {
            Some(LutClut::parse(
                &data[clut_offset..],
                input_channels,
                output_channels,
            )?)
        } else {
            None
        };

        let a_curves = if a_offset != 0 {
            Some(parse_curve_set(data, a_offset, output_channels as usize)?)
        } else {
            None
        };

        Ok(Self {
            input_channels,
            output_channels,
            b_curves,
            matrix,
            m_curves,
            clut,
            a_curves,
        })
    }
}

/// Matrix element in LUT (3x3 + 3 offset)
#[derive(Debug, Clone)]
pub struct LutMatrix {
    /// 3x3 matrix
    pub matrix: [[f64; 3]; 3],
    /// 3 offset values
    pub offset: [f64; 3],
}

impl LutMatrix {
    /// Parse matrix from bytes
    pub fn parse(data: &[u8]) -> Result<Self, IccError> {
        if data.len() < 48 {
            return Err(IccError::CorruptedData("LUT matrix too small".to_string()));
        }

        let mut matrix = [[0.0f64; 3]; 3];
        for row in 0..3 {
            for col in 0..3 {
                let offset = (row * 3 + col) * 4;
                let raw = i32::from_be_bytes([
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ]);
                matrix[row][col] = raw as f64 / 65536.0;
            }
        }

        let mut offs = [0.0f64; 3];
        for i in 0..3 {
            let offset = 36 + i * 4;
            let raw = i32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offs[i] = raw as f64 / 65536.0;
        }

        Ok(Self {
            matrix,
            offset: offs,
        })
    }
}

/// CLUT element in LUT
#[derive(Debug, Clone)]
pub struct LutClut {
    /// Grid points per dimension (up to 16 dimensions supported by ICC)
    pub grid_points: Vec<u8>,
    /// Precision: 1 for u8, 2 for u16
    pub precision: u8,
    /// CLUT data (normalized to 0.0-1.0)
    pub data: Vec<f64>,
    /// Number of output channels
    pub output_channels: u8,
}

impl LutClut {
    /// Parse CLUT from bytes
    pub fn parse(data: &[u8], input_channels: u8, output_channels: u8) -> Result<Self, IccError> {
        if data.len() < 20 {
            return Err(IccError::CorruptedData("LUT CLUT too small".to_string()));
        }

        // Grid points for each dimension (16 bytes, only first input_channels used)
        let mut grid_points = Vec::with_capacity(input_channels as usize);
        for i in 0..input_channels as usize {
            grid_points.push(data[i]);
        }

        let precision = data[16];
        // data[17..20] reserved

        // Calculate number of CLUT entries
        let mut total_entries = 1usize;
        for &g in &grid_points {
            total_entries *= g as usize;
        }
        total_entries *= output_channels as usize;

        let data_offset = 20;
        let bytes_per_entry = precision as usize;
        let required_bytes = total_entries * bytes_per_entry;

        if data.len() < data_offset + required_bytes {
            return Err(IccError::CorruptedData(
                "LUT CLUT data truncated".to_string(),
            ));
        }

        // Parse CLUT data
        let mut clut_data = Vec::with_capacity(total_entries);
        for i in 0..total_entries {
            let offset = data_offset + i * bytes_per_entry;
            let value = if precision == 1 {
                data[offset] as f64 / 255.0
            } else {
                let v = u16::from_be_bytes([data[offset], data[offset + 1]]);
                v as f64 / 65535.0
            };
            clut_data.push(value);
        }

        Ok(Self {
            grid_points,
            precision,
            data: clut_data,
            output_channels,
        })
    }
}

/// Curve segment in v4 LUTs (can be curv or para type)
#[derive(Debug, Clone)]
pub enum CurveSegment {
    /// Identity curve
    Identity,
    /// Table curve
    Table(Vec<f64>),
    /// Parametric curve
    Parametric { curve_type: u16, params: Vec<f64> },
}

/// Parse a set of curves from data
fn parse_curve_set(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<CurveSegment>, IccError> {
    let mut curves = Vec::with_capacity(count);
    let mut pos = offset;

    for _ in 0..count {
        if pos + 8 > data.len() {
            return Err(IccError::CorruptedData("Curve set truncated".to_string()));
        }

        let type_sig = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        // Bytes 4-7 are reserved

        pos += 8;

        match &type_sig.to_be_bytes() {
            b"curv" => {
                if pos + 4 > data.len() {
                    return Err(IccError::CorruptedData("curv header truncated".to_string()));
                }

                let count =
                    u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                        as usize;
                pos += 4;

                let curve = if count == 0 {
                    CurveSegment::Identity
                } else if count == 1 {
                    if pos + 2 > data.len() {
                        return Err(IccError::CorruptedData("curv gamma truncated".to_string()));
                    }
                    let gamma_raw = u16::from_be_bytes([data[pos], data[pos + 1]]);
                    let gamma = gamma_raw as f64 / 256.0;
                    pos += 2;
                    CurveSegment::Parametric {
                        curve_type: 0,
                        params: vec![gamma],
                    }
                } else {
                    let required = count * 2;
                    if pos + required > data.len() {
                        return Err(IccError::CorruptedData("curv table truncated".to_string()));
                    }

                    let mut table = Vec::with_capacity(count);
                    for i in 0..count {
                        let v = u16::from_be_bytes([data[pos + i * 2], data[pos + i * 2 + 1]]);
                        table.push(v as f64 / 65535.0);
                    }
                    pos += required;
                    CurveSegment::Table(table)
                };

                curves.push(curve);
            }
            b"para" => {
                if pos + 4 > data.len() {
                    return Err(IccError::CorruptedData("para header truncated".to_string()));
                }

                let func_type = u16::from_be_bytes([data[pos], data[pos + 1]]);
                // Bytes 2-3 reserved
                pos += 4;

                // Number of parameters depends on function type
                let param_count = match func_type {
                    0 => 1, // g
                    1 => 3, // g, a, b
                    2 => 4, // g, a, b, c
                    3 => 5, // g, a, b, c, d
                    4 => 7, // g, a, b, c, d, e, f
                    _ => 0,
                };

                let required = param_count * 4;
                if pos + required > data.len() {
                    return Err(IccError::CorruptedData("para params truncated".to_string()));
                }

                let mut params = Vec::with_capacity(param_count);
                for i in 0..param_count {
                    let raw = i32::from_be_bytes([
                        data[pos + i * 4],
                        data[pos + i * 4 + 1],
                        data[pos + i * 4 + 2],
                        data[pos + i * 4 + 3],
                    ]);
                    params.push(raw as f64 / 65536.0);
                }
                pos += required;

                curves.push(CurveSegment::Parametric {
                    curve_type: func_type,
                    params,
                });
            }
            _ => {
                // Unknown curve type - skip (can't know size)
                return Err(IccError::CorruptedData(format!(
                    "Unknown curve type in set: {:08X}",
                    type_sig
                )));
            }
        }

        // Align to 4-byte boundary
        pos = (pos + 3) & !3;
    }

    Ok(curves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut8_basic_parse() {
        // Minimal Lut8 header with 3x3 identity matrix
        let mut data = vec![0u8; 48];
        data[0] = 3; // input channels
        data[1] = 3; // output channels
        data[2] = 2; // grid points

        // Identity matrix (s15Fixed16: 1.0 = 0x00010000)
        let one = 0x00010000u32.to_be_bytes();
        let zero = [0u8; 4];

        // Row 0
        data[4..8].copy_from_slice(&one);
        data[8..12].copy_from_slice(&zero);
        data[12..16].copy_from_slice(&zero);
        // Row 1
        data[16..20].copy_from_slice(&zero);
        data[20..24].copy_from_slice(&one);
        data[24..28].copy_from_slice(&zero);
        // Row 2
        data[28..32].copy_from_slice(&zero);
        data[32..36].copy_from_slice(&zero);
        data[36..40].copy_from_slice(&one);

        // Input tables (3 channels * 256 bytes = 768)
        data.extend(vec![0u8; 768]);
        for i in 0..3 {
            for j in 0..256 {
                data[40 + i * 256 + j] = j as u8;
            }
        }

        // CLUT (2^3 * 3 = 24 bytes)
        data.extend(vec![128u8; 24]);

        // Output tables (3 channels * 256 bytes)
        for _ in 0..3 {
            for j in 0..256 {
                data.push(j as u8);
            }
        }

        let lut = Lut8Data::parse(&data).unwrap();
        assert_eq!(lut.input_channels, 3);
        assert_eq!(lut.output_channels, 3);
        assert_eq!(lut.grid_points, 2);
        assert!(lut.matrix_is_identity());
    }
}
