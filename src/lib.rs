mod compress;
mod decompress;
mod error;
mod format;

pub use crate::compress::lz11::{Strategy, compress, compress_lz};
pub use crate::decompress::{decompress};
pub use crate::error::LZError;
pub use crate::format::Format;
