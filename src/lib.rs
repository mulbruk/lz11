use byteorder::{LittleEndian, ReadBytesExt};
use core::hash;
use std::io::{Cursor, Read};

use error::LZError;

pub mod error;

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

pub fn decompress(data: &[u8]) -> Result<Vec<u8>, LZError> {
  let mut cursor = Cursor::new(data);

  // Read 4 bytes into a u8 array
  let mut header = [0u8; 4];
  cursor
    .read_exact(&mut header)
    .map_err(|_| LZError::HeaderTooShort)?;

  // The first byte is the magic number
  let format = Format::try_from(header[0])?;

  // The next 3 bytes possibly represent the decompressed size in little-endian format
  let possible_size = (&header[1..4]).read_u24::<LittleEndian>().unwrap();

  let decompressed_size = if possible_size == 0 && format == Format::LZ11 {
    cursor
      .read_u32::<LittleEndian>()
      .map_err(|_| LZError::HeaderTooShort)?
  } else {
    possible_size
  } as usize;

  let mut result: Vec<u8> = Vec::with_capacity(decompressed_size);

  let flag_masks: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];
  while result.len() < decompressed_size {
    let flag_byte = cursor.read_u8().map_err(|_| LZError::DataTooShort)?;
    for &mask in &flag_masks {
      if result.len() >= decompressed_size {
        break;
      }

      if (flag_byte & mask) == 0 {
        // Uncompressed byte
        let byte = cursor.read_u8().map_err(|_| LZError::DataTooShort)?;
        result.push(byte);
      } else {
        // Compressed block
        let (length, displacement) = match format {
          Format::LZ10 => {
            // 2-byte encoding: LLLL DDDD DDDD DDDD
            let byte1 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;
            let byte2 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;

            let length = ((byte1 & 0xF0) >> 4) + 3;
            let displacement = ((byte1 & 0x0F) << 8) + byte2;
            (length, displacement)
          }
          Format::LZ11 => {
            // LZ11 uses variable length encoding based on the upper nybble of the first byte
            let byte1 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;
            let encoding = (byte1 & 0xF0) >> 4;

            match encoding {
              0 => {
                // 3-byte encoding: 0000 LLLL DDDD DDDD DDDD DDDD
                let byte2 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;
                let byte3 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;

                let length = (((byte1 & 0x0F) << 4) + ((byte2 & 0xF0) >> 4)) + 0x11;
                let displacement = ((byte2 & 0x0F) << 8) + byte3;
                (length, displacement)
              }
              1 => {
                // 4-byte encoding 0000 LLLL LLLL LLLL LLLL DDDD DDDD DDDD
                let byte2 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;
                let byte3 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;
                let byte4 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;

                let length =
                  (((byte1 & 0x0F) << 12) + (byte2 << 4) + ((byte3 & 0xF0) >> 4)) + 0x111;
                let displacement = ((byte3 & 0x0F) << 8) + byte4;
                (length, displacement)
              }
              _ => {
                // 2-byte encoding: LLLL DDDD DDDD DDDD
                let byte2 = cursor.read_u8().map_err(|_| LZError::DataTooShort)? as usize;

                let length = ((byte1 & 0xF0) >> 4) + 1;
                let displacement = ((byte1 & 0x0F) << 8) + byte2;
                (length, displacement)
              }
            }
          }
        };

        let offset = result.len() - displacement - 1;
        if length < result.len() - offset {
          result.extend_from_within(offset..offset + length);
        } else {
          for n in 0..length {
            let byte = result[offset + n];
            result.push(byte);
          }
        }
      }
    }
  }

  Ok(result)
}

// LZ11 Compression --------------------------------------------

const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;

const LZ11_MIN_MATCH_LENGTH: usize = 3;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111

/// Returns offset and length of the longest match for the given offset, or None if no match of at least 3 bytes is found.
fn naive_find_longest_match(data: &[u8], offset: usize) -> Option<(usize, usize)> {
  if offset < 4 || data.len() < 4 {
    return None;
  }

  let mut longest_match = 0;
  let mut match_start = if offset < 0x1000 { 0 } else { offset - 0x1000 };

  for n in match_start..offset {
    let mut match_length = 0;
    
    // while offset + match_length < data.len() && n + match_length < offset && data[n + match_length] == data[offset + match_length] {
    while offset + match_length < data.len() && match_length < LZ11_MAX_MATCH_LENGTH && data[n + match_length] == data[offset + match_length] {
      match_length += 1;
    }

    if match_length > longest_match {
      longest_match = match_length;
      match_start = n;
    }
  }

  if longest_match < LZ11_MIN_MATCH_LENGTH {
    // println!("No match found for offset {}: longest match length is {}", offset, longest_match);
    None
  } else {
    // println!("Match found for offset {}: longest match length is {}, match start is {}", offset, longest_match, match_start);
    Some((match_start, longest_match))
  } 
}

const WINDOW_SIZE: usize = 0x1000; // 4096 byte sliding window
const HASH_BITS: usize = 12; // 4096 possible hash values
const HASH_SIZE: usize = 1 << HASH_BITS; // 4096
const HASH_MASK: usize = HASH_SIZE - 1; // 4095
const HASH_MAX_CHAIN: usize = 256; // Limit the number of candidates to check for a match to avoid worst-case performance

struct HashMatcher {
  head: Vec<usize>,
  prev: Vec<usize>,
}

impl HashMatcher {
  fn new() -> Self {
    HashMatcher {
      // head[hash] gives the most recent offset where a 3-byte sequence with that hash was seen, or usize::MAX if none
      head: vec![usize::MAX; HASH_SIZE],

      // ring buffer, prev[offset % WINDOW_SIZE] gives the previous offset where the same 3-byte sequence was seen, or usize::MAX if none
      prev: vec![usize::MAX; WINDOW_SIZE],
    }
  }

  #[inline]
  fn hash(data: &[u8], offset: usize) -> usize {
    if offset + 2 >= data.len() {
      return 0;
    }
    let b1 = data[offset] as usize;
    let b2 = data[offset + 1] as usize;
    let b3 = data[offset + 2] as usize;
    ((b1 << 8) ^ (b2 << 4) ^ b3) & HASH_MASK
  }

  fn insert(&mut self, data: &[u8], offset: usize) {
    if offset + 2 >= data.len() {
      return;
    }

    let hash_value = Self::hash(data, offset);
    self.prev[offset % WINDOW_SIZE] = self.head[hash_value];
    self.head[hash_value] = offset;
  }

  /// Returns offset and length of the longest match for the given offset, or None if no match of at least 3 bytes is found.
  /// The search is limited to the sliding window and the maximum match length.
  fn find_longest_match(&self, data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset < LZ11_MIN_MATCH_LENGTH || data.len() < offset + LZ11_MIN_MATCH_LENGTH {
      return None;
    }

    let hash_value = Self::hash(data, offset);
    let mut match_candidate = self.head[hash_value];

    let lowest_position = offset.saturating_sub(WINDOW_SIZE);
    let match_limit = LZ11_MAX_MATCH_LENGTH.min(data.len() - offset);

    let mut longest_match = 0;
    let mut match_start = 0;
    let mut steps = 0;

    while match_candidate != usize::MAX && match_candidate >= lowest_position && steps < HASH_MAX_CHAIN {
      // Skip over self-references
      if match_candidate >= offset {
        match_candidate = self.prev[match_candidate % WINDOW_SIZE];
        steps += 1;
        continue;
      }

      let mut len = 0;
      while len < match_limit && data[match_candidate + len] == data[offset + len] {
        len += 1;
      }

      if len > longest_match {
        longest_match = len;
        match_start = match_candidate;

        if longest_match == match_limit {
          break; // Can't get a longer match, so stop searching
        }
      }

      match_candidate = self.prev[match_candidate % WINDOW_SIZE];
      steps += 1;
    }

    if longest_match < LZ11_MIN_MATCH_LENGTH {
      None
    } else {
      Some((match_start, longest_match))
    }
  }
}

pub enum CompressionMethod {
  Naive,
  HashChain,
}

pub fn compress_lz11(data: &[u8], method: CompressionMethod) -> Result<Vec<u8>, LZError> {
  match method {
    CompressionMethod::Naive => compress_lz11_naive(data),
    CompressionMethod::HashChain => compress_lz11_hash_chain(data),
  }
}

fn compress_lz11_naive(data: &[u8]) -> Result<Vec<u8>, LZError> {
  if data.len() > LZ11_MAX_INPUT_LENGTH {
    return Err(LZError::InputTooLarge);
  }

  // Compressed data
  let mut result: Vec<u8> = Vec::new();

  let mut flag_byte: u8 = 0;
  let mut blocks: Vec<u8> = Vec::new();
  let mut block_index: usize = 0;

  let flag_masks: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

  // Write header
  result.push(Format::LZ11 as u8);
  if data.len() < 0x1000000 {
    result.extend_from_slice(&(data.len() as u32).to_le_bytes()[..3]);
  } else {
    result.extend_from_slice(&[0, 0, 0]);
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
  }

  // Write compressed data
  let mut n = 0;
  while n < data.len() {
    if let Some((match_start, match_length)) = naive_find_longest_match(data, n) {
      // Compressed block
      let length = match_length;
      let displacement = n - match_start - 1;
      
      if match_length >= 0x111 {
        // 4-byte encoding
        let block: [u8; 4] = [
          0x10 | ((length - 0x111) >> 12) as u8,
          ((length - 0x111) >> 4) as u8,
          (((length - 0x111) << 4) as u8) | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        flag_byte |= flag_masks[block_index];
        blocks.extend_from_slice(&block);
      } else if match_length >= 0x11 {
        // 3-byte encoding
        let block: [u8; 3] = [
          ((length - 0x11) >> 4) as u8,
          (((length - 0x11) << 4) as u8) | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        flag_byte |= flag_masks[block_index];
        blocks.extend_from_slice(&block);
      } else {
        // 2-byte encoding
        let block: [u8; 2] = [
          ((length - 1) << 4) as u8 | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        flag_byte |= flag_masks[block_index];
        blocks.extend_from_slice(&block);
      }

      n += match_length;
    } else {
      // Uncompressed byte
      blocks.push(data[n]);
      
      n += 1;
    }

    block_index += 1;
    if block_index >= 8 {
      result.push(flag_byte);
      result.extend_from_slice(&blocks);
      flag_byte = 0;
      blocks.clear();
      block_index = 0;
    }
  }

  // Flush remaining blocks
  if block_index > 0 {
    result.push(flag_byte);
    result.extend_from_slice(&blocks);
  }
  result.push(0xff);

  Ok(result)
}

fn compress_lz11_hash_chain(data: &[u8]) -> Result<Vec<u8>, LZError> {
  if data.len() > LZ11_MAX_INPUT_LENGTH {
    return Err(LZError::InputTooLarge);
  }

  // Compressed data
  let mut result: Vec<u8> = Vec::new();

  let mut flag_byte: u8 = 0;
  let mut blocks: Vec<u8> = Vec::new();
  let mut block_index: usize = 0;

  let flag_masks: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

  // Write header
  result.push(Format::LZ11 as u8);
  if data.len() < 0x1000000 {
    result.extend_from_slice(&(data.len() as u32).to_le_bytes()[..3]);
  } else {
    result.extend_from_slice(&[0, 0, 0]);
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
  }

  let mut matcher = HashMatcher::new();

  // Write compressed data
  let mut n = 0;
  while n < data.len() {
    matcher.insert(data, n);
    if let Some((match_start, match_length)) = matcher.find_longest_match(data, n) {
      // Compressed block
      let length = match_length;
      let displacement = n - match_start - 1;
      
      if match_length >= 0x111 {
        // 4-byte encoding
        let block: [u8; 4] = [
          0x10 | ((length - 0x111) >> 12) as u8,
          ((length - 0x111) >> 4) as u8,
          (((length - 0x111) << 4) as u8) | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        flag_byte |= flag_masks[block_index];
        blocks.extend_from_slice(&block);
      } else if match_length >= 0x11 {
        // 3-byte encoding
        let block: [u8; 3] = [
          ((length - 0x11) >> 4) as u8,
          (((length - 0x11) << 4) as u8) | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        flag_byte |= flag_masks[block_index];
        blocks.extend_from_slice(&block);
      } else {
        // 2-byte encoding
        let block: [u8; 2] = [
          ((length - 1) << 4) as u8 | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        flag_byte |= flag_masks[block_index];
        blocks.extend_from_slice(&block);
      }

      for skipped in 1..match_length {
        // if n + skipped < data.len() {
          matcher.insert(data, n + skipped);
        // }
      }
      n += match_length;
    } else {
      // Uncompressed byte
      blocks.push(data[n]);
      
      n += 1;
    }

    block_index += 1;
    if block_index >= 8 {
      result.push(flag_byte);
      result.extend_from_slice(&blocks);
      flag_byte = 0;
      blocks.clear();
      block_index = 0;
    }
  }

  // Flush remaining blocks
  if block_index > 0 {
    result.push(flag_byte);
    result.extend_from_slice(&blocks);
  }
  result.push(0xff);

  Ok(result)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_works() {
    let result = 2 + 2;
    assert_eq!(result, 4);
  }
}
