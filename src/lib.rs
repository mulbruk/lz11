mod compress;
mod decompress;
mod error;
mod format;

pub use crate::compress::lz11::{LZ11Strategy, compress_lz11};
pub use crate::decompress::{decompress};
pub use crate::error::LZError;
pub use crate::format::Format;

// LZ11 Compression --------------------------------------------


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
  }
}
