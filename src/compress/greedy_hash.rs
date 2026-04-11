use crate::error::LZError;
use crate::format::Format;

use super::hash_matcher::HashMatcher;
use super::lz11_context::LZ11Context;

const LZ11_MAX_INPUT_LENGTH: usize = 0xFFFFFFFF;
const LZ11_MIN_MATCH_LENGTH: usize = 3;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111


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
    matcher.insert(data, n);
    if let Some((match_start, match_length)) = matcher.find_longest_match(data, n) {
      lz11_context.write_compressed_block(n, match_start, match_length, &mut result);
      
      for skipped in 1..match_length {
          matcher.insert(data, n + skipped);
      }
      n += match_length;
    } else {
      lz11_context.write_uncompressed_byte(data[n], &mut result);
      
      n += 1;
    }
  }

  // Flush remaining blocks
  lz11_context.flush(&mut result);
  result.push(0xff);

  Ok(result)
}
