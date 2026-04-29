use crate::error::LZError;
use crate::format::Format;

use super::hash_matcher::HashMatcher;

// Constants ---------------------------------------------------
const LZ10_MAX_INPUT_LENGTH: usize = 0xFFFFFF;
const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;

const LZ_MIN_MATCH_LENGTH: usize = 3;

const FLAG_MASKS: [u8; 8] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];

// LZ Context --------------------------------------------------
struct LZContext {
  format: Format,
  flag_byte: u8,
  blocks: Vec<u8>,
  block_index: usize,
}

impl LZContext {
  fn new(format: Format) -> Self {
    LZContext {
      format,
      flag_byte: 0,
      blocks: Vec::new(),
      block_index: 0,
    }
  }

  fn write_compressed_block(&mut self, offset: usize, match_start: usize, match_length: usize, result: &mut Vec<u8>) {
    match self.format {
      Format::LZ10 => {
        // Compressed block
        let length = match_length;
        let displacement = offset - match_start - 1;
        
        // 2-byte encoding
        let block: [u8; 2] = [
          ((length - 3) << 4) as u8 | ((displacement >> 8) as u8),
          (displacement & 0xFF) as u8,
        ];

        self.flag_byte |= FLAG_MASKS[self.block_index];
        self.blocks.extend_from_slice(&block);
      },
      Format::LZ11 => {
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
      },
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

// Compression -------------------------------------------------
#[derive(Clone, Copy)]
pub enum Strategy {
  Greedy,
  Lazy,
  Optimal,
}

pub fn compress_lz(data: &[u8], format: Format, strategy: Strategy, max_chain: usize) -> Result<Vec<u8>, LZError> {
  if (format == Format::LZ10 && data.len() > LZ10_MAX_INPUT_LENGTH) || (format == Format::LZ11 && data.len() > LZ11_MAX_INPUT_LENGTH) {
    return Err(LZError::InputTooLarge);
  }

  // Compressed data
  let mut result: Vec<u8> = Vec::new();

  // Write header
  result.push(format as u8);
  if data.len() < 0x1000000 {
    result.extend_from_slice(&(data.len() as u32).to_le_bytes()[..3]);
  } else {
    result.extend_from_slice(&[0, 0, 0]);
    result.extend_from_slice(&(data.len() as u32).to_le_bytes());
  }

  match strategy {
    Strategy::Greedy => compress_greedy(data, format, max_chain, &mut result),
    Strategy::Lazy => compress_lazy(data, format, max_chain, &mut result),
    Strategy::Optimal => compress_optimal(data, format, &mut result),
  }

  // Write trailing 0xff for compatibility with other LZ10/LZ11 tool implementations
  result.push(0xff);

  Ok(result)
}

// Compression Interface ---------------------------------------
fn compression_level(level: usize) -> Result<(Strategy, usize), LZError> {
  match level {
    1 => Ok((Strategy::Greedy, 64)),
    2 => Ok((Strategy::Greedy, 128)),
    3 => Ok((Strategy::Greedy, 256)),
    4 => Ok((Strategy::Greedy, 512)),
    5 => Ok((Strategy::Lazy, 64)),
    6 => Ok((Strategy::Lazy, 128)),
    7 => Ok((Strategy::Lazy, 256)),
    8 => Ok((Strategy::Lazy, 512)),
    9 => Ok((Strategy::Optimal, 0)),
    _ => Err(LZError::InvalidCompressionLevel(level)),
  }
}

pub fn compress(data: &[u8], format: Format, level: usize) -> Result<Vec<u8>, LZError> {
  let (strategy, max_chain) = compression_level(level)?;
  compress_lz(data, format, strategy, max_chain)
}

// Greedy Hash Chain -------------------------------------------
fn compress_greedy(data: &[u8], format: Format, max_chain: usize, result: &mut Vec<u8>) {
  let mut lz_context = LZContext::new(format);
  let mut matcher = HashMatcher::new(format, max_chain);

  // Write compressed data
  let mut n = 0;
  while n < data.len() {
    matcher.insert(data, n);
    if let Some((match_start, match_length)) = matcher.find_longest_match(data, n) {
      lz_context.write_compressed_block(n, match_start, match_length, result);
      
      for skipped in 1..match_length {
          matcher.insert(data, n + skipped);
      }
      n += match_length;
    } else {
      lz_context.write_uncompressed_byte(data[n], result);
      
      n += 1;
    }
  }

  // Flush remaining blocks
  lz_context.flush(result);
}

// Lazy Hash Chain ---------------------------------------------
fn compress_lazy(data: &[u8], format: Format, max_chain: usize, result: &mut Vec<u8>) {
  let mut lz_context = LZContext::new(format);
  let mut matcher = HashMatcher::new(format, max_chain);

  // Write compressed data
  let mut n = 0;
  while n < data.len() {
    if n >= data.len() - 3 {
      // Not enough bytes left for a match, so just write uncompressed bytes
      lz_context.write_uncompressed_byte(data[n], result);
      n += 1;
      continue;
    } else {
      matcher.insert(data, n);
      let match_1 = matcher.find_longest_match(data, n);
      matcher.insert(data, n + 1);
      let match_2 = matcher.find_longest_match(data, n + 1);

      match (match_1, match_2) {
        (None, None) => {
          lz_context.write_uncompressed_byte(data[n], result);
          lz_context.write_uncompressed_byte(data[n + 1], result);
          n += 2;
        },
        
        (Some((match_start, match_length)), None) => {
          lz_context.write_compressed_block(n, match_start, match_length, result);
          for skipped in 2..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (None, Some((match_start, match_length))) => {
          lz_context.write_uncompressed_byte(data[n], result);
          n += 1;
          
          lz_context.write_compressed_block(n, match_start, match_length, result);
          for skipped in 1..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (Some((match_start_1, match_length_1)), Some((match_start_2, match_length_2))) => {
          if match_length_2 > match_length_1 {
            lz_context.write_uncompressed_byte(data[n], result);
            n += 1;
            
            lz_context.write_compressed_block(n, match_start_2, match_length_2, result);
            for skipped in 1..match_length_2 {
              matcher.insert(data, n + skipped);
            }
            n += match_length_2;
          } else {
            lz_context.write_compressed_block(n, match_start_1, match_length_1, result);
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
  lz_context.flush(result);
}

// Optimal Parsing ---------------------------------------------
fn compress_optimal(data: &[u8], format: Format, result: &mut Vec<u8>) {
  let mut lz_context = LZContext::new(format);
  let choices = optimal_parse(data, format);

  let mut n = 0;
  for choice in choices.into_iter() {
    match choice {
      Choice::Literal => {
        // Literal byte
        lz_context.write_uncompressed_byte(data[n], result);
        n += 1;
      }
      Choice::Reference { length, offset } => {
        lz_context.write_compressed_block(n, offset, length, result);
        n += length;
      }
    }
  }
  
  lz_context.flush(result);
}

#[derive(Clone, Copy)]
enum Choice {
  Literal,
  Reference { length: usize, offset: usize },
}

fn encoding_cost(format: Format, length: usize) -> usize {
  // Cost in bits for each encoding option
  // 1 flag bit is always consumed regardless of encoding type
  if length == 0 {
    // Literal: 1 flag bit + 8 data bits
    9
  } else {
    match format {
      Format::LZ10 => 17, 
      Format::LZ11 => if length <= 16 {
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
  }
}

fn optimal_parse(data: &[u8], format: Format) -> Vec<Choice> {
  let data_len = data.len();

  let mut matcher = HashMatcher::new(format, 4096);

  let mut costs = vec![usize::MAX; data_len + 1];
  let mut choices = vec![Choice::Literal; data_len + 1];
  costs[0] = 0;

  for n in 0..data_len {
    if costs[n] == usize::MAX {
      continue;
    }

    // Option 1: literal at position n
    let literal_cost = costs[n] + encoding_cost(format, 0);
    if literal_cost < costs[n + 1] {
      costs[n + 1] = literal_cost;
      choices[n + 1] = Choice::Literal;
    }

    // Option 2: find best match at position n
    if let Some((match_start, match_length)) = matcher.find_longest_match(data, n) {
      let min_length: usize = LZ_MIN_MATCH_LENGTH;                                                                                                                                                      
      let max_length: usize = match_length.min(data_len - 1);
                                                                                                                                                                                                        
      for length in min_length..=max_length {
          let ref_cost = costs[n] + encoding_cost(format, length);                                                                                                                                              
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
