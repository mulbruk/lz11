use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Cursor, Read};

use crate::error::LZError;
use crate::format::Format;

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