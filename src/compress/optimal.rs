use crate::error::LZError;
use crate::format::Format;

use super::hash_matcher::HashMatcher;

const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;

const LZ11_MIN_MATCH_LENGTH: usize = 3;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111

const WINDOW_SIZE: usize = 0x1000; // 4096 byte sliding window
const HASH_BITS: usize = 12; // 4096 possible hash values
const HASH_SIZE: usize = 1 << HASH_BITS; // 4096
const HASH_MASK: usize = HASH_SIZE - 1; // 4095
const HASH_MAX_CHAIN: usize = 4096; // Limit the number of candidates to check for a match to avoid worst-case performance

#[derive(Clone, Copy)]
enum Choice {
  Literal,
  Reference { length: usize, offset: usize },
}

fn encoding_cost(length: usize) -> usize {
  // Cost in bits for each encoding option
  // 1 flag bit is always consumed regardless of encoding type
  if length == 0 {
    // Literal: 1 flag bit + 8 data bits
    9
  } else if length <= 16 {
    // 2-byte reference
    17
  } else if length <= 272 {
    // 3-byte reference
    25
  } else {
    // 4-byte reference
    33
  }
}

fn optimal_parse(data: &[u8]) -> Vec<Choice> {
  let data_len = data.len();

  let mut matcher = HashMatcher::new();

  let mut costs = vec![usize::MAX; data_len + 1];
  let mut choices = vec![Choice::Literal; data_len + 1];
  costs[0] = 0;

  for n in 0..data_len {
    if costs[n] == usize::MAX {
      continue;
    }

    // Option 1: literal at position n
    let literal_cost = costs[n] + encoding_cost(0);
    if literal_cost < costs[n + 1] {
      costs[n + 1] = literal_cost;
      choices[n + 1] = Choice::Literal;
    }

    // Option 2: find all matches at position n using hash chains
    let matches = matcher.find_matches(data, n);

    for (match_start, match_length) in matches {
      let min_length: usize = LZ11_MIN_MATCH_LENGTH;
      let max_length: usize = match_length.min(data_len - 1);

      for length in min_length..=max_length {
        let ref_cost = costs[n] + encoding_cost(length);
        let target = n + length;

        if ref_cost < costs[target] {
          costs[target] = ref_cost;
          choices[target] = Choice::Reference { length, offset: match_start };
        }
      }

    }

    matcher.insert(data, n);
  }

  // Walk backwards to reconstruct the optimal sequence
  let mut result: Vec<Choice> = Vec::new();
  let mut pos = data_len;
  while pos > 0 {
    match choices[pos] {
      Choice::Literal => {
        result.push(Choice::Literal);
        pos -= 1;
      }
      Choice::Reference { length, offset } => {
        result.push(Choice::Reference { length, offset });
        pos -= length;
      }
    }
  }

  result.reverse();
  result
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
    // println!("offset: {}, match_start: {}, match_length: {}", offset, match_start, match_length);
    
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

  // Write compressed data
  let choices = optimal_parse(data);

  let mut n = 0;
  for choice in choices.into_iter() {
    match choice {
      Choice::Literal => {
        // Literal byte
        compressor_state.write_uncompressed_byte(data[n], &mut result);
        n += 1;
      }
      Choice::Reference { length, offset } => {
        compressor_state.write_compressed_block(n, offset, length, &mut result);
        n += length;
      }
    }
  }
  
  compressor_state.flush(&mut result);
  result.push(0xff);

  Ok(result)
}
