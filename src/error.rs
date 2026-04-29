use thiserror::Error;

/// Errors that can occur during LZ10/LZ11 compression/decompression.
#[derive(Error, Debug)]
pub enum LZError {
  /// The first byte of the input is not a recognized format identifier (`0x10` or `0x11`).
  #[error("invalid magic number: expected 0x10 or 0x11, got {0:#04x}")]
  InvalidMagicNumber(u8),

  /// The input data is too short to contain a valid LZ10/LZ11 header.
  #[error("data too short to contain a valid LZ10/LZ11 header")]
  HeaderTooShort,

  /// The compressed data ended unexpectedly before the full output could be produced.
  #[error("data too short")]
  DataTooShort,

  /// The input data exceeds the maximum size for the chosen format.
  #[error("input data too large")]
  InputTooLarge,

  /// The compression level is not in the valid range (1-9).
  #[error("invalid compression level: {0}")]
  InvalidCompressionLevel(usize),
}
