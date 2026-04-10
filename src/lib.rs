pub mod compress;
pub mod decompress;
pub mod error;
pub mod format;

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
