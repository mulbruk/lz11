use crate::error::LZError;

#[cfg(feature = "cli")]
use clap::ValueEnum;

/// The compression format to use (LZ10 or LZ11).
///
/// LZ10 supports data up to 2^24 bytes in length, and encodes references using a fixed 2-byte
/// format supporting match lengths of 3-18 bytes.
/// 
/// LZ11 extends this to support data up to 2^32 bytes in length, with variable-length encoding
/// (2-4 bytes) supporting match lengths up to 65,808 bytes. This generally produces better
/// compression ratios than LZ10.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[repr(u8)]
pub enum Format {
  /// LZ10 format (magic byte `0x10`). Fixed 2-byte reference encoding, max input size ~16 MiB.
  LZ10 = 0x10,
  /// LZ11 format (magic byte `0x11`). Variable-length reference encoding, max input size ~4 GiB.
  LZ11 = 0x11,
}

impl TryFrom<u8> for Format {
  type Error = LZError;

  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0x10 => Ok(Format::LZ10),
      0x11 => Ok(Format::LZ11),
      _ => Err(LZError::InvalidMagicNumber(value)),
    }
  }
}
