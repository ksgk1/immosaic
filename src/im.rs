use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use image::{ImageReader, Rgba, RgbaImage, imageops};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use walkdir::{DirEntry, WalkDir};

/// Thumbnail is practically identical to the full image's average.
const AVG_THUMBNAIL_SIZE: u32 = 256;

/// The file name in the folder that holds the image information.
const IMAGE_DB_FILE_NAME: &str = "mosaic.json";

/// The supported image formats.
const SUPPORTED_FILE_TYPE: [&str; 5] = ["bmp", "jpeg", "jpg", "png", "webp"];

fn is_supported_file(entry: &DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(|s| {
        SUPPORTED_FILE_TYPE
            .iter()
            .any(|ext| s.to_ascii_lowercase().ends_with(ext))
    })
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|s| s.starts_with('.'))
}

fn get_image_paths(src: &PathBuf) -> Vec<PathBuf> {
    WalkDir::new(src)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(std::result::Result::ok)
        .filter(is_supported_file)
        .map(walkdir::DirEntry::into_path)
        .collect()
}

fn load_rgba(path: &Path) -> Option<RgbaImage> {
    ImageReader::open(path)
        .and_then(|reader| reader.decode().map_err(std::io::Error::other))
        .map_err(|e| eprintln!("skipping {}: {e}", path.display()))
        .ok()
        .map(|img| img.to_rgba8())
}

fn average_color(img: &RgbaImage) -> Rgba<u8> {
    let zero = ([0u64; 4], 0u64);
    let (sum, count) = img
        .par_pixels()
        .fold(
            || zero,
            |(mut acc, mut cnt), p| {
                let Rgba([r, g, b, a]) = *p;
                acc[0] += u64::from(r);
                acc[1] += u64::from(g);
                acc[2] += u64::from(b);
                acc[3] += u64::from(a);
                cnt += 1;
                (acc, cnt)
            },
        )
        .reduce(
            || zero,
            |(mut a, mut ca), (b, cb)| {
                for i in 0..4 {
                    a[i] += b[i];
                }
                ca += cb;
                (a, ca)
            },
        );

    if count == 0 {
        return Rgba([0, 0, 0, 0]);
    }

    Rgba([
        (sum[0] / count) as u8,
        (sum[1] / count) as u8,
        (sum[2] / count) as u8,
        (sum[3] / count) as u8,
    ])
}

type Cache = HashMap<String, [u8; 4]>;

pub fn generate_cache(src: &Path, regenerate: bool) -> Result<Cache, Box<dyn std::error::Error>> {
    let cache_file_location = src.join(IMAGE_DB_FILE_NAME);
    if cache_file_location.exists() && !regenerate {
        println!(
            "Cache file already exists, using existing data. \
             If the data is incomplete, re-run with -b flag."
        );
        let content = std::fs::read_to_string(&cache_file_location)
            .map_err(|e| format!("cannot read cache file: {e}"))?;
        let cache: Cache =
            serde_json::from_str(&content).map_err(|e| format!("cache file is corrupt: {e}"))?;
        return Ok(cache);
    }

    let paths = get_image_paths(&src.to_path_buf());
    let pb = ProgressBar::new(paths.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("Creating cache [{bar:40.cyan/blue}] ({percent}%) {msg}")
            .expect("invalid progress template")
            .progress_chars("#>-"),
    );

    let cache: Cache = paths
        .into_par_iter()
        .filter_map(|path| {
            let img = load_rgba(&path)?;
            let thumb = imageops::thumbnail(&img, AVG_THUMBNAIL_SIZE, AVG_THUMBNAIL_SIZE);
            let avg = average_color(&thumb);
            pb.inc(1);
            Some((path.to_string_lossy().into_owned(), avg.0))
        })
        .collect();

    if cache.is_empty() {
        return Err("Could not find any images for creating the cache. Exiting.".into());
    }

    let json = serde_json::to_string_pretty(&cache)?;
    std::fs::write(&cache_file_location, json)?;
    pb.finish_with_message("done");

    println!(
        "Created cache file \"{}\" with {} entries.",
        cache_file_location.display(),
        cache.len()
    );

    Ok(cache)
}

fn rgba_difference(lhs: [u8; 4], rhs: [u8; 4]) -> u32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(a, b)| (i32::from(*a) - i32::from(*b)).unsigned_abs())
        .sum()
}

fn find_closest_match(cache: &Cache, value: Rgba<u8>) -> &str {
    assert!(!cache.is_empty());
    cache
        .iter()
        .min_by_key(|(_, v)| rgba_difference(value.0, **v))
        .map(|(k, _)| k.as_str())
        .expect("cache must not be empty")
}

/// geometry + which cache image it resolved to.
struct Tile {
    x: i64,
    y: i64,
    width: u32,
    height: u32,
    key: String,
}

pub fn generate_mosaic(
    cache: &Cache,
    input_image: &Path,
    output: &Path,
    grid_items: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    if cache.is_empty() {
        return Err("Cannot create image without cache.".into());
    }
    let img = load_rgba(input_image)
        .ok_or_else(|| format!("failed to load image: {}", input_image.display()))?;
    let (width, height) = img.dimensions();

    let total = u64::from(grid_items).pow(2);
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("Generating     [{bar:40.cyan/blue}] ({percent}%) {msg}")
            .expect("invalid progress template")
            .progress_chars("#>-"),
    );

    let tiles: Vec<(i64, i64, u32, u32, Rgba<u8>)> = (0..grid_items)
        .into_par_iter()
        .flat_map(|row| (0..grid_items).into_par_iter().map(move |col| (row, col)))
        .map(|(row, col)| {
            let x0 = col * width / grid_items;
            let x1 = (col + 1) * width / grid_items;
            let y0 = row * height / grid_items;
            let y1 = (row + 1) * height / grid_items;
            let (tile_width, tile_height) = (x1 - x0, y1 - y0);

            let tile = imageops::crop_imm(&img, x0, y0, tile_width, tile_height);
            let avg = average_color(&tile.to_image());

            (i64::from(x0), i64::from(y0), tile_width, tile_height, avg)
        })
        .collect();

    let mut runtime_cache = HashMap::<Rgba<u8>, String>::new();
    let tiles: Vec<Tile> = tiles
        .into_iter()
        .map(|(x, y, width, height, avg)| {
            let key = runtime_cache
                .entry(avg)
                .or_insert_with(|| find_closest_match(cache, avg).to_owned())
                .clone();
            Tile {
                x,
                y,
                width,
                height,
                key,
            }
        })
        .collect();

    let wanted: HashSet<(String, u32, u32)> = tiles
        .iter()
        .map(|t| (t.key.clone(), t.width, t.height))
        .collect();
    let pb_load = ProgressBar::new(wanted.len() as u64);
    pb_load.set_style(
        ProgressStyle::with_template("Loading data   [{bar:40.cyan/blue}]")
            .expect("invalid progress template")
            .progress_chars("#>-"),
    );

    let scaled: HashMap<(String, u32, u32), RgbaImage> = wanted
        .into_par_iter()
        .filter_map(|(key, tw, th)| {
            let img = load_rgba(Path::new(&key))?;
            let s = imageops::resize(&img, tw, th, imageops::FilterType::Triangle);
            pb_load.inc(1);
            Some(((key, tw, th), s))
        })
        .collect();
    pb_load.finish();

    let mut canvas = RgbaImage::new(width, height);
    for tile in &tiles {
        let s = scaled
            .get(&(tile.key.clone(), tile.width, tile.height))
            .ok_or_else(|| format!("failed to load mosaic image: {}", tile.key))?;
        imageops::overlay(&mut canvas, s, tile.x, tile.y);
        pb.inc(1);
    }
    pb.finish_with_message("done");
    canvas.save(output)?;
    Ok(())
}
