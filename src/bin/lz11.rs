use std::fs;
use std::path::PathBuf;
use std::process;

use clap::Parser;

use lz11::{
  Format, LZError, LZ11Strategy,
  compress_lz11, decompress,
};

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
    /// Compression strategy (greedy, lazy, optimal)
    #[arg(short, long, default_value = "greedy")]
    strategy: LZ11Strategy,
  },
  /// Decompress an LZ10 or LZ11 compressed file
  Decompress {
    /// Input file path
    input: PathBuf,
    /// Output file path
    output: PathBuf,
  },
}

fn cmd_decompress(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  let data = fs::read(input)?;
  let decompressed_data = decompress(&data)?;
  fs::write(output, decompressed_data)?;
  Ok(())
}

fn cmd_compress(input: &PathBuf, output: &PathBuf, strategy: LZ11Strategy) -> Result<(), Box<dyn std::error::Error>> {
  let data = fs::read(input)?;
  let compressed_data = compress_lz11(&data, strategy)?;
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
    Commands::Compress { input, output, strategy } => {
      if let Err(e) = cmd_compress(&input, &output, strategy) {
        eprintln!("Error: {}", e);
        process::exit(1);
      }
    }
  }
}
