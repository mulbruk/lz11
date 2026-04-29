//! Library for handling Nintendo's LZ10 and LZ11 compressionformats.
//!
//! These formats are used for asset compression in Nintendo DS, GBA, and Wii titles.
//! (Maybe also WiiU? idk).
//! 
//! LZ10 uses a fixed 2-byte reference encoding. LZ11 extends the format with support for
//! larger files and uses a variable-length encoding that achieves better compression ratios.
//!
//! # Example
//!
//! ```
//! use lz11::{compress, decompress, Format};
//!
//! let data = b"Dear Mario: Please come to the castle. I've baked a cake for you. Yours truly -- Princess Toadstool (Peach)";
//! let compressed = compress(data, Format::LZ11, 5).unwrap();
//! let decompressed = decompress(&compressed).unwrap();
//! assert_eq!(data.as_slice(), &decompressed);
//! ```

mod compress;
mod decompress;
mod error;
mod format;

pub use crate::compress::lz11::compress;
pub use crate::decompress::decompress;
pub use crate::error::LZError;
pub use crate::format::Format;
