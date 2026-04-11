pub mod greedy_hash;
pub mod optimal;
pub mod lazy_hash;
pub mod naive;

use crate::error::LZError;

pub enum CompressionMethod {
  Naive,
  HashChain,
  LazyHash,
  Optimal,
}

pub fn compress_lz11(data: &[u8], method: CompressionMethod) -> Result<Vec<u8>, LZError> {
  match method {
    CompressionMethod::Naive => naive::compress_lz11(data),
    CompressionMethod::HashChain => greedy_hash::compress_lz11(data),
    CompressionMethod::LazyHash => lazy_hash::compress_lz11(data),
    CompressionMethod::Optimal => optimal::compress_lz11(data),
  }
}
