use std::process::exit;

use crate::{
    cli::parse_args,
    im::{generate_cache, generate_mosaic},
};

mod cli;
mod im;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let src = args.folder_sources;
    if !src.is_dir() {
        eprintln!(
            "{} is not a directly. Exiting.",
            src.into_string().expect("Path is valid string.")
        );
        exit(1);
    }
    let cache = generate_cache(&src, args.build_cache)?;

    let input_image = args.input_image;
    generate_mosaic(
        &cache,
        &input_image,
        &args.output_path,
        u32::from(args.tiles_per_side),
    )
}
