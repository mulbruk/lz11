use crate::format::Format;

// Constants ---------------------------------------------------
const LZ_MIN_MATCH_LENGTH: usize = 3;
const LZ10_MAX_MATCH_LENGTH: usize = 18;
const LZ11_MAX_MATCH_LENGTH: usize = 65808; // (2^16 - 1) + 0x111

const WINDOW_SIZE: usize = 0x1000; // 4096 byte sliding window
const HASH_BITS: usize = 12; // 4096 possible hash values -> log2(4096) bits
const HASH_SIZE: usize = 1 << HASH_BITS; // 4096
const HASH_MASK: usize = HASH_SIZE - 1; // 4095

// Matcher -----------------------------------------------------
pub(crate) struct HashMatcher {
  head: Vec<usize>,
  prev: Vec<usize>,

  max_match: usize,
  max_chain: usize,
}

impl HashMatcher {
  pub fn new(format: Format, max_chain: usize) -> Self {
    let max_match = match format {
      Format::LZ10 => LZ10_MAX_MATCH_LENGTH,
      Format::LZ11 => LZ11_MAX_MATCH_LENGTH,
    };

    HashMatcher {
      // head[hash] gives the most recent offset where a 3-byte sequence with that hash was seen, or usize::MAX if none
      head: vec![usize::MAX; HASH_SIZE],

      // ring buffer, prev[offset % WINDOW_SIZE] gives the previous offset where the same 3-byte sequence was seen, or usize::MAX if none
      prev: vec![usize::MAX; WINDOW_SIZE],

      max_match,
      max_chain,
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

  pub fn insert(&mut self, data: &[u8], offset: usize) {
    if offset + 2 >= data.len() {
      return;
    }

    let hash_value = Self::hash(data, offset);
    self.prev[offset % WINDOW_SIZE] = self.head[hash_value];
    self.head[hash_value] = offset;
  }

  /// Returns offset and length of the longest match for the given offset, or None if no match of at least 3 bytes is found.
  /// The search is limited to the sliding window and the maximum match length.
  pub fn find_longest_match(&self, data: &[u8], offset: usize) -> Option<(usize, usize)> {
    if offset < LZ_MIN_MATCH_LENGTH || data.len() < offset + LZ_MIN_MATCH_LENGTH {
      return None;
    }

    let hash_value = Self::hash(data, offset);
    let mut match_candidate = self.head[hash_value];

    let lowest_position = offset.saturating_sub(WINDOW_SIZE);
    let match_limit = self.max_match.min(data.len() - offset);

    let mut longest_match = 0;
    let mut match_start = 0;
    let mut steps = 0;

    while match_candidate != usize::MAX
      && match_candidate >= lowest_position
      && steps < self.max_chain
    {
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

    if longest_match < LZ_MIN_MATCH_LENGTH {
      None
    } else {
      Some((match_start, longest_match))
    }
  }
}
