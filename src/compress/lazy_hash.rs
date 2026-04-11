use crate::error::LZError;
use crate::format::Format;

const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;

const LZ11_MIN_MATCH_LENGTH: usize = 3;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111

use super::hash_matcher::HashMatcher;
use super::lz11_context::LZ11Context;

pub(crate) fn compress_lz11(data: &[u8]) -> Result<Vec<u8>, LZError> {
  if data.len() > LZ11_MAX_INPUT_LENGTH {
    return Err(LZError::InputTooLarge);
  }

  // Compressed data
  let mut result: Vec<u8> = Vec::new();

  let mut lz11_context = LZ11Context::new();

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
      lz11_context.write_uncompressed_byte(data[n], &mut result);
      n += 1;
      continue;
    } else {
      matcher.insert(data, n);
      let match_1 = matcher.find_longest_match(data, n);
      matcher.insert(data, n + 1);
      let match_2 = matcher.find_longest_match(data, n + 1);

      match (match_1, match_2) {
        (None, None) => {
          lz11_context.write_uncompressed_byte(data[n], &mut result);
          lz11_context.write_uncompressed_byte(data[n + 1], &mut result);
          n += 2;
        },
        
        (Some((match_start, match_length)), None) => {
          lz11_context.write_compressed_block(n, match_start, match_length, &mut result);
          for skipped in 2..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (None, Some((match_start, match_length))) => {
          lz11_context.write_uncompressed_byte(data[n], &mut result);
          n += 1;
          
          lz11_context.write_compressed_block(n, match_start, match_length, &mut result);
          for skipped in 1..match_length {
            matcher.insert(data, n + skipped);
          }
          n += match_length;
        },

        (Some((match_start_1, match_length_1)), Some((match_start_2, match_length_2))) => {
          if match_length_2 > match_length_1 {
            lz11_context.write_uncompressed_byte(data[n], &mut result);
            n += 1;
            
            lz11_context.write_compressed_block(n, match_start_2, match_length_2, &mut result);
            for skipped in 1..match_length_2 {
              matcher.insert(data, n + skipped);
            }
            n += match_length_2;
          } else {
            lz11_context.write_compressed_block(n, match_start_1, match_length_1, &mut result);
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
  lz11_context.flush(&mut result);
  result.push(0xff);

  Ok(result)
}
