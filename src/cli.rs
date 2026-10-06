use std::path::PathBuf;

pub struct Args {
    pub folder_sources: PathBuf,
    pub input_image:    PathBuf,
    pub output_path:    PathBuf,
    pub rebuild_cache:  bool,
    pub tiles_per_side: u8,
    pub scale:          u8,
}

pub fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut folder_sources = PathBuf::new();
    let mut input_image = PathBuf::new();
    let mut output_path = PathBuf::new();
    let mut rebuild_cache = false;
    let mut tiles_per_side = 10;
    let mut scale = 1;

    while let Some(arg) = parser.next()? {
        match arg {
            Long("sources") => {
                folder_sources = parser.value()?.parse()?;
            }
            Long("input-image") => {
                input_image = parser.value()?.parse()?;
            }
            Long("output") => {
                output_path = parser.value()?.parse()?;
            }
            Long("rebuild-cache") => {
                rebuild_cache = true;
            }
            Long("tiles-per-side") => {
                tiles_per_side = parser.value()?.parse()?;
            }
            Long("scale") => {
                scale = parser.value()?.parse()?;
                scale = scale.clamp(1, 10);
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
        rebuild_cache,
        tiles_per_side,
        scale,
    })
}
