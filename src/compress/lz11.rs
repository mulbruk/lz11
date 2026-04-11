use crate::error::LZError;
use crate::format::Format;

#[cfg(feature = "cli")]
use clap::ValueEnum;

use super::hash_matcher::HashMatcher;

// LZ11 Constants ----------------------------------------------
const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;
const LZ11_MIN_MATCH_LENGTH: usize = 3;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111

// LZ11 Context ------------------------------------------------
const FLAG_MASKS: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

struct LZ11Context {
  flag_byte: u8,
  blocks: Vec<u8>,
  block_index: usize,
}

impl LZ11Context {
  fn new() -> Self {
    LZ11Context {
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

// LZ11 Compression --------------------------------------------
#[derive(Clone, Copy)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum LZ11Strategy {
  Greedy,
  Lazy,
  Optimal,
}

pub fn compress_lz11(data: &[u8], strategy: LZ11Strategy) -> Result<Vec<u8>, LZError> {
  if data.len() > LZ11_MAX_INPUT_LENGTH {
    return Err(LZError::InputTooLarge);
  }

  // Compressed data
  let mut result: Vec<u8> = Vec::new();

  // Write header
  result.push(Format::LZ11 as u8);
  if data.len() < 0x1000000 {
    result.extend_from_slice(&(data.len() as u32).to_le_bytes()[..3]);
  } else {
    result.extend_from_slice(&[0, 0, 0]);
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
  }

  match strategy {
    LZ11Strategy::Greedy => compress_lz11_greedy(data, &mut result),
    LZ11Strategy::Lazy => compress_lz11_lazy(data, &mut result),
    LZ11Strategy::Optimal => compress_lz11_optimal(data, &mut result),
  }

  // Write footer
  result.push(0xff);

  Ok(result)
}

// Greedy Hash Chain -------------------------------------------
fn compress_lz11_greedy(data: &[u8], result: &mut Vec<u8>) {
  let mut lz11_context = LZ11Context::new();
  let mut matcher = HashMatcher::new();

  // Write compressed data
  let mut n = 0;
  while n < data.len() {
    matcher.insert(data, n);
    if let Some((match_start, match_length)) = matcher.find_longest_match(data, n) {
      lz11_context.write_compressed_block(n, match_start, match_length, &mut *result);
      
      for skipped in 1..match_length {
          matcher.insert(data, n + skipped);
      }
      n += match_length;
    } else {
      lz11_context.write_uncompressed_byte(data[n], &mut *result);
      
      n += 1;
    }
  }

  // Flush remaining blocks
  lz11_context.flush(&mut *result);
}

// Lazy Hash Chain ---------------------------------------------
fn compress_lz11_lazy(data: &[u8], result: &mut Vec<u8>) {
  let mut lz11_context = LZ11Context::new();
  let mut matcher = HashMatcher::new();

  // Write compressed data
  let mut n = 0;
  while n < data.len() {
    if n >= data.len() - 3 {
      // Not enough bytes left for a match, so just write uncompressed bytes
      lz11_context.write_uncompressed_byte(data[n], &mut *result);
      n += 1;
      continue;
    } else {
      matcher.insert(data, n);
      let match_1 = matcher.find_longest_match(data, n);
      matcher.insert(data, n + 1);
      let match_2 = matcher.find_longest_match(data, n + 1);

      match (match_1, match_2) {
        (None, None) => {
          lz11_context.write_uncompressed_byte(data[n], &mut *result);
          lz11_context.write_uncompressed_byte(data[n + 1], &mut *result);
          n += 2;
        },
        
        (Some((match_start, match_length)), None) => {
          lz11_context.write_compressed_block(n, match_start, match_length, &mut *result);
          for skipped in 2..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (None, Some((match_start, match_length))) => {
          lz11_context.write_uncompressed_byte(data[n], &mut *result);
          n += 1;
          
          lz11_context.write_compressed_block(n, match_start, match_length, &mut *result);
          for skipped in 1..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (Some((match_start_1, match_length_1)), Some((match_start_2, match_length_2))) => {
          if match_length_2 > match_length_1 {
            lz11_context.write_uncompressed_byte(data[n], &mut *result);
            n += 1;
            
            lz11_context.write_compressed_block(n, match_start_2, match_length_2, &mut *result);
            for skipped in 1..match_length_2 {
              matcher.insert(data, n + skipped);
            }
            n += match_length_2;
          } else {
            lz11_context.write_compressed_block(n, match_start_1, match_length_1, &mut *result);
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
  lz11_context.flush(&mut *result);
}

// Optimal Parsing ---------------------------------------------
fn compress_lz11_optimal(data: &[u8], result: &mut Vec<u8>) {
  let mut lz11_context = LZ11Context::new();
  let choices = optimal_parse(data);

  let mut n = 0;
  for choice in choices.into_iter() {
    match choice {
      Choice::Literal => {
        // Literal byte
        lz11_context.write_uncompressed_byte(data[n], &mut *result);
        n += 1;
      }
      Choice::Reference { length, offset } => {
        lz11_context.write_compressed_block(n, offset, length, &mut *result);
        n += length;
      }
    }
  }
  
  lz11_context.flush(&mut *result);
}

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
