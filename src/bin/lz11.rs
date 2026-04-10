use std::fs;
use std::path::PathBuf;
use std::process;

use clap::Parser;

#[derive(Parser)]
struct Args {
  #[command(subcommand)]
  command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
  /// Decompress an LZ10 or LZ11 compressed file
  Decompress {
    /// Input file path
    input: PathBuf,
    /// Output file path
    output: PathBuf,
  },
}

fn decompress(input: &PathBuf, output: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  let data = fs::read(input)?;
  let decompressed_data = lz11::decompress(&data)?;
  fs::write(output, decompressed_data)?;
  Ok(())
}

fn main() {
  let args = Args::parse();

  match args.command {
    Commands::Decompress { input, output } => {
      if let Err(e) = decompress(&input, &output) {
        eprintln!("Error: {}", e);
        process::exit(1);
      }
    }
  }
}
