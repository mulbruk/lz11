use hex_literal::hex;
use sha2::{Sha512, Digest}; 
use lz11::decompress;

const BOOK2_HASH: [u8; 64] = hex!("588326a8264cf990cb6c06e1d205ec00e63eca22ada419b67dc6b00da2518901633b93c27ca1a9e20155dca106f5ee6711bfaf8e13ce142b80ecd7a149705d01");

#[test]
fn decompress_lz10() {
  let data_lz10 = std::fs::read("resources/book2.lz10").expect("Failed to read book2.lz10");
  let data = decompress(&data_lz10).expect("Decompression failed for book2.lz10");
  let hash = Sha512::digest(&data);

  assert_eq!(hash, BOOK2_HASH);
}

#[test]
fn decompress_lz11() {
  let data_lz11 = std::fs::read("resources/book2.lz11").expect("Failed to read book2.lz11");
  let data = decompress(&data_lz11).expect("Decompression failed for book2.lz11");
  let hash = Sha512::digest(&data);

  assert_eq!(hash, BOOK2_HASH);
}
