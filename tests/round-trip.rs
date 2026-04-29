use lz11::{compress, decompress, Format};

macro_rules! round_trip_test_files {
  ($($name:ident: $format:expr, $file:expr), *) => {
    $(
      #[test]
      fn $name() {
        let data = std::fs::read(concat!("resources/calgary-corpus/", $file)).expect("Failed to read input file");
        let compressed = compress(&data, $format, 5).expect(&format!("Compression failed for file {}", $file));
        let decompressed = decompress(&compressed).expect(&format!("Decompression failed for file {}", $file));
        assert_eq!(data, decompressed, "Round-trip failed for file {}", $file);
      }
    )*
  };
}

macro_rules! round_trip_test_levels {
  ($($name:ident: $format:expr, $level:expr), *) => {
    $(
      #[test]
      fn $name() {
        let data = std::fs::read("resources/calgary-corpus/obj2").expect("Failed to read input file");
        let compressed = compress(&data, $format, $level).expect(&format!("Compression failed for compression level {}", $level));
        let decompressed = decompress(&compressed).expect(&format!("Decompression failed for compression level {}", $level));
        assert_eq!(data, decompressed, "Round-trip failed at compression level {}", $level);
      }
    )*
  };
}

round_trip_test_files!(
  round_trip_lz10_bib:    Format::LZ10, "bib",
  round_trip_lz10_book1:  Format::LZ10, "book1",
  round_trip_lz10_book2:  Format::LZ10, "book2",
  round_trip_lz10_geo:    Format::LZ10, "geo",
  round_trip_lz10_news:   Format::LZ10, "news",
  round_trip_lz10_obj1:   Format::LZ10, "obj1",
  round_trip_lz10_obj2:   Format::LZ10, "obj2",
  round_trip_lz10_paper1: Format::LZ10, "paper1",
  round_trip_lz10_paper2: Format::LZ10, "paper2",
  round_trip_lz10_pic:    Format::LZ10, "pic",
  round_trip_lz10_progc:  Format::LZ10, "progc",
  round_trip_lz10_progl:  Format::LZ10, "progl",
  round_trip_lz10_progp:  Format::LZ10, "progp",
  round_trip_lz10_trans:  Format::LZ10, "trans"
);

round_trip_test_files!(
  round_trip_lz11_bib:    Format::LZ11, "bib",
  round_trip_lz11_book1:  Format::LZ11, "book1",
  round_trip_lz11_book2:  Format::LZ11, "book2",
  round_trip_lz11_geo:    Format::LZ11, "geo",
  round_trip_lz11_news:   Format::LZ11, "news",
  round_trip_lz11_obj1:   Format::LZ11, "obj1",
  round_trip_lz11_obj2:   Format::LZ11, "obj2",
  round_trip_lz11_paper1: Format::LZ11, "paper1",
  round_trip_lz11_paper2: Format::LZ11, "paper2",
  round_trip_lz11_pic:    Format::LZ11, "pic",
  round_trip_lz11_progc:  Format::LZ11, "progc",
  round_trip_lz11_progl:  Format::LZ11, "progl",
  round_trip_lz11_progp:  Format::LZ11, "progp",
  round_trip_lz11_trans:  Format::LZ11, "trans"
);

round_trip_test_levels!(
  round_trip_lz10_obj2_o1: Format::LZ10, 1,
  round_trip_lz11_obj2_o1: Format::LZ11, 1,
  round_trip_lz10_obj2_o2: Format::LZ10, 2,
  round_trip_lz11_obj2_o2: Format::LZ11, 2,
  round_trip_lz10_obj2_o3: Format::LZ10, 3,
  round_trip_lz11_obj2_o3: Format::LZ11, 3,
  round_trip_lz10_obj2_o4: Format::LZ10, 4,
  round_trip_lz11_obj2_o4: Format::LZ11, 4,
  round_trip_lz10_obj2_o5: Format::LZ10, 5,
  round_trip_lz11_obj2_o5: Format::LZ11, 5,
  round_trip_lz10_obj2_o6: Format::LZ10, 6,
  round_trip_lz11_obj2_o6: Format::LZ11, 6,
  round_trip_lz10_obj2_o7: Format::LZ10, 7,
  round_trip_lz11_obj2_o7: Format::LZ11, 7,
  round_trip_lz10_obj2_o8: Format::LZ10, 8,
  round_trip_lz11_obj2_o8: Format::LZ11, 8,
  round_trip_lz10_obj2_o9: Format::LZ10, 9,
  round_trip_lz11_obj2_o9: Format::LZ11, 9
);
