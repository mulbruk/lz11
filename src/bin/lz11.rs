use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use clap::Parser;

use lz11::{Format, compress, decompress};

#[derive(Parser)]
struct Args {
  #[command(subcommand)]
  command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
  /// Compress a file using LZ11 compression
  Compress {
    /// Input file path
    input: PathBuf,
    /// Output file path
    output: PathBuf,
    /// Compression format (lz10 or lz11)
    #[arg(short, long, default_value = "lz11")]
    format: Format,
    /// Compression level (1-9)
    #[arg(short = 'o', long, default_value_t = 5)]
    level: usize,
  },
  /// Decompress an LZ10 or LZ11 compressed file
  Decompress {
    /// Input file path
    input: PathBuf,
    /// Output file path
    output: PathBuf,
  },
}

fn cmd_decompress(input: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
  let data = fs::read(input)?;
  let decompressed_data = decompress(&data)?;
  fs::write(output, decompressed_data)?;
  Ok(())
}

fn cmd_compress(
  input: &Path,
  output: &Path,
  format: Format,
  level: usize,
) -> Result<(), Box<dyn std::error::Error>> {
  let data = fs::read(input)?;
  let compressed_data = compress(&data, format, level)?;
  fs::write(output, compressed_data)?;
  Ok(())
}

fn main() {
  let args = Args::parse();

  match args.command {
    Commands::Decompress { input, output } => {
      if let Err(e) = cmd_decompress(&input, &output) {
        eprintln!("Error: {}", e);
        process::exit(1);
      }
    }
    Commands::Compress {
      input,
      output,
      format,
      level,
    } => {
      if let Err(e) = cmd_compress(&input, &output, format, level) {
        eprintln!("Error: {}", e);
        process::exit(1);
      }
    }
  }
}
