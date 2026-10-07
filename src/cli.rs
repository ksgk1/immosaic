use std::path::PathBuf;
use std::process;

pub struct Args {
    pub folder_sources: PathBuf,
    pub input_image:    PathBuf,
    pub output_path:    PathBuf,
    pub rebuild_cache:  bool,
    pub tiles_per_side: u8,
    pub scale:          u8,
}

pub fn parse_args() -> Result<Args, String> {
    let args = std::env::args().collect::<Vec<_>>();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let mut folder_sources = None;
    let mut input_image = None;
    let mut output_path = None;
    let mut rebuild_cache = false;
    let mut tiles_per_side = 10;
    let mut scale = 1;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--sources" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --sources".to_string());
                }
                folder_sources = Some(PathBuf::from(&args[i]));
            }
            "--input-image" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --input-image".to_string());
                }
                input_image = Some(PathBuf::from(&args[i]));
            }
            "--output" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --output".to_string());
                }
                output_path = Some(PathBuf::from(&args[i]));
            }
            "--rebuild-cache" => {
                rebuild_cache = true;
            }
            "--tiles-per-side" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --tiles-per-side".to_string());
                }
                tiles_per_side = args[i].parse().map_err(|_| "Invalid tiles-per-side value".to_string())?;
            }
            "--scale" => {
                i += 1;
                if i >= args.len() {
                    return Err("Missing value for --scale".to_string());
                }
                scale = args[i].parse().map_err(|_| "Invalid scale value".to_string())?;
            }
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            "--version" | "-v" => {
                println!("immosaic {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            _ => {
                if args[i].starts_with("--") {
                    return Err(format!("Unknown option: {}", args[i]));
                }
            }
        }
        i += 1;
    }

    let folder_sources = folder_sources.ok_or_else(|| "folder_sources is required".to_string())?;
    let input_image = input_image.ok_or_else(|| "input_image is required".to_string())?;
    let output_path = output_path.ok_or_else(|| "output_path is required".to_string())?;

    Ok(Args {
        folder_sources,
        input_image,
        output_path,
        rebuild_cache,
        tiles_per_side,
        scale,
    })
}

fn print_usage() {
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "immosaic".to_string());

    println!("Usage: {exe} --sources <path> --input-image <path> --output <path> [--tiles-per-side N] [--scale N] [--rebuild-cache] [--version]");
    println!();
    println!("Options:");
    println!("  --sources <path>           Source image directory");
    println!("  --input-image <path>       Input image to create mosaic from");
    println!("  --output <path>            Output mosaic image path");
    println!("  --tiles-per-side N         Number of tiles per side (default: 10)");
    println!("  --scale N                  Scale factor for output (default: 1)");
    println!("  --rebuild-cache            Force rebuild of image cache");
    println!("  --help, -h                 Show this help message");
    println!("  --version, -v              Show version information");
}
