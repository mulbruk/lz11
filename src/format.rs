use crate::error::LZError;

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Format {
  LZ10 = 0x10,
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
