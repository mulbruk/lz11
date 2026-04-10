use crate::error::LZError;
use crate::format::Format;

const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;

const LZ11_MIN_MATCH_LENGTH: usize = 3;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111

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

const FLAG_MASKS: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

struct CompressorState {
  flag_byte: u8,
  blocks: Vec<u8>,
  block_index: usize,
}

impl CompressorState {
  fn new() -> Self {
    CompressorState {
      flag_byte: 0,
      blocks: Vec::new(),
      block_index: 0,
    }
  }

  fn write_compressed_block(&mut self, offset: usize, match_start: usize, match_length: usize, result: &mut Vec<u8>) {
    // Compressed block
    let length = match_length;
    let displacement = offset - match_start - 1;
    
    if match_length >= 0x111 {
      // 4-byte encoding
      let block: [u8; 4] = [
        0x10 | ((length - 0x111) >> 12) as u8,
        ((length - 0x111) >> 4) as u8,
        (((length - 0x111) << 4) as u8) | ((displacement >> 8) as u8),
        (displacement & 0xFF) as u8,
      ];

      self.flag_byte |= FLAG_MASKS[self.block_index];
      self.blocks.extend_from_slice(&block);
    } else if match_length >= 0x11 {
      // 3-byte encoding
      let block: [u8; 3] = [
        ((length - 0x11) >> 4) as u8,
        (((length - 0x11) << 4) as u8) | ((displacement >> 8) as u8),
        (displacement & 0xFF) as u8,
      ];

      self.flag_byte |= FLAG_MASKS[self.block_index];
      self.blocks.extend_from_slice(&block);
    } else {
      // 2-byte encoding
      let block: [u8; 2] = [
        ((length - 1) << 4) as u8 | ((displacement >> 8) as u8),
        (displacement & 0xFF) as u8,
      ];

      self.flag_byte |= FLAG_MASKS[self.block_index];
      self.blocks.extend_from_slice(&block);
    }

    self.block_index += 1;
    if self.block_index >= 8 {
      result.push(self.flag_byte);
      result.extend_from_slice(&self.blocks);
      self.flag_byte = 0;
      self.blocks.clear();
      self.block_index = 0;
    }
  }

  fn write_uncompressed_byte(&mut self, byte: u8, result: &mut Vec<u8>) {
    self.blocks.push(byte);

    self.block_index += 1;
    if self.block_index >= 8 {
      result.push(self.flag_byte);
      result.extend_from_slice(&self.blocks);
      self.flag_byte = 0;
      self.blocks.clear();
      self.block_index = 0;
    }
  }

  fn flush(&mut self, result: &mut Vec<u8>) {
    if self.block_index > 0 {
      result.push(self.flag_byte);
      result.extend_from_slice(&self.blocks);
      self.flag_byte = 0;
      self.blocks.clear();
      self.block_index = 0;
    }
  }
}

pub(crate) fn compress_lz11(data: &[u8]) -> Result<Vec<u8>, LZError> {
  if data.len() > LZ11_MAX_INPUT_LENGTH {
    return Err(LZError::InputTooLarge);
  }

  // Compressed data
  let mut result: Vec<u8> = Vec::new();

  let mut compressor_state = CompressorState::new();

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
    
    if n >= data.len() - 3 {
      // Not enough bytes left for a match, so just write uncompressed bytes
      compressor_state.write_uncompressed_byte(data[n], &mut result);
      n += 1;
      continue;
    } else {
      matcher.insert(data, n);
      let match_1 = matcher.find_longest_match(data, n);
      matcher.insert(data, n + 1);
      let match_2 = matcher.find_longest_match(data, n + 1);

      match (match_1, match_2) {
        (None, None) => {
          compressor_state.write_uncompressed_byte(data[n], &mut result);
          compressor_state.write_uncompressed_byte(data[n + 1], &mut result);
          n += 2;
        },
        
        (Some((match_start, match_length)), None) => {
          compressor_state.write_compressed_block(n, match_start, match_length, &mut result);
          for skipped in 2..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (None, Some((match_start, match_length))) => {
          compressor_state.write_uncompressed_byte(data[n], &mut result);
          n += 1;
          
          compressor_state.write_compressed_block(n, match_start, match_length, &mut result);
          for skipped in 1..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (Some((match_start_1, match_length_1)), Some((match_start_2, match_length_2))) => {
          if match_length_2 > match_length_1 {
            compressor_state.write_uncompressed_byte(data[n], &mut result);
            n += 1;
            
            compressor_state.write_compressed_block(n, match_start_2, match_length_2, &mut result);
            for skipped in 1..match_length_2 {
              matcher.insert(data, n + skipped);
            }
            n += match_length_2;
          } else {
            compressor_state.write_compressed_block(n, match_start_1, match_length_1, &mut result);
            for skipped in 2..match_length_1 {
              matcher.insert(data, n + skipped);
            }
            n += match_length_1;
          }
        },
      }
    }
  }

  // Flush remaining blocks
  compressor_state.flush(&mut result);
  result.push(0xff);

  Ok(result)
}
