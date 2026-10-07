use std::process::exit;

use crate::cli::parse_args;
use crate::im::Cache;

mod cli;
mod im;
mod progress;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let src = &args.folder_sources;
    if !src.is_dir() {
        eprintln!("{} is not a directory. Exiting.", src.display());
        exit(1);
    }
    let cache = Cache::generate_from_path(src, args.rebuild_cache)?;
    cache.generate_mosaic(&args.input_image, &args.output_path, u32::from(args.tiles_per_side), u32::from(args.scale))
}
