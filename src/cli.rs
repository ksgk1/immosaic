use std::path::PathBuf;

pub struct Args {
    pub folder_sources: PathBuf,
    pub input_image:    PathBuf,
    pub output_path:    PathBuf,
    pub build_cache:    bool,
    pub tiles_per_side: u8,
}

pub fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut folder_sources = PathBuf::new();
    let mut input_image = PathBuf::new();
    let mut output_path = PathBuf::new();
    let mut build_cache = false;
    let mut parser = lexopt::Parser::from_env();
    let mut tiles_per_side = 10;

    while let Some(arg) = parser.next()? {
        match arg {
            Short('s') | Long("sources") => {
                folder_sources = parser.value()?.parse()?;
            }
            Short('i') | Long("input-image") => {
                input_image = parser.value()?.parse()?;
            }
            Short('o') | Long("output") => {
                output_path = parser.value()?.parse()?;
            }
            Short('b') | Long("build-cache") => {
                build_cache = true;
            }
            Short('t') | Long("tiles-per-side") => {
                tiles_per_side = parser.value()?.parse()?;
            }
            Long("help") => {
                let exe = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.file_name().and_then(|n| n.to_str().map(String::from)))
                    .unwrap_or_else(|| String::from("immosaic"));
                println!("Usage: {exe} --sources <path-to-source-image-dir> --output mosaic.png --input-image source.jpg --tiles-per-side 50");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        folder_sources,
        input_image,
        output_path,
        build_cache,
        tiles_per_side,
    })
}
