use thiserror::Error;

#[derive(Error, Debug)]
pub enum LZError {
  #[error("invalid magic number: expected 0x10 or 0x11, got {0:#04x}")]
  InvalidMagicNumber(u8),

  #[error("data too short to contain a valid LZ10/LZ11 header")]
  HeaderTooShort,

  #[error("data too short")]
  DataTooShort,

  #[error("input data too large")]
  InputTooLarge,
}
