use crate::error::LZError;
use crate::format::Format;

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

pub(crate) fn compress_lz11(data: &[u8]) -> Result<Vec<u8>, LZError> {
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
