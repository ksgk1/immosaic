use std::{
    collections::HashMap,
    fs::File,
    io::read_to_string,
    path::{Path, PathBuf},
};

use image::{ImageReader, Rgba, RgbaImage, imageops};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::iter::ParallelIterator as _;
use walkdir::{DirEntry, WalkDir};

const IMAGE_DB_FILE_NAME: &str = "mosaic.json";
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

pub fn generate_cache(
    src: &PathBuf,
    regenerate: bool,
) -> Result<Cache, Box<dyn std::error::Error>> {
    let mut cache = Cache::new();
    let cache_file_location = src.join(IMAGE_DB_FILE_NAME);
    if cache_file_location.exists() && !regenerate {
        println!(
            "Cache file already exists, using existing data. If the data is incomplete, re-run with -b flag."
        );
        let cache_file_conent =
            read_to_string(File::open(cache_file_location)?).expect("File has compatible format.");
        let cache: Cache = serde_json::from_str(&cache_file_conent)?;
        return Ok(cache);
    }
    for path in get_image_paths(src) {
        if let Some(image) = load_rgba(&path) {
            let avg = average_color(&image);
            cache.insert(path.to_str().expect("non-UTF-8 path").to_owned(), avg.0);
        }
    }
    if cache.is_empty() {
        eprintln!("No files found for cache.");
        return Err("Could not find any images for creating the cache. Exiting.".into());
    }
    let json = serde_json::to_string_pretty(&cache)?;
    std::fs::write(&cache_file_location, json)?;
    println!(
        "Created cache file \"{}\" with {} entries.",
        cache_file_location.into_string().expect("Valid path"),
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
        ProgressStyle::with_template("[{bar:40.cyan/blue}] ({percent}%) {msg}")
            .expect("invalid progress template")
            .progress_chars("#>-"),
    );

    let mut canvas = RgbaImage::new(width, height);
    let mut runtime_cache = HashMap::<Rgba<u8>, String>::new();
    let mut image_cache = HashMap::<String, RgbaImage>::new();
    let mut scaled_cache = HashMap::<(String, u32, u32), RgbaImage>::new();

    for row in 0..grid_items {
        for col in 0..grid_items {
            let x0 = col * width / grid_items;
            let x1 = (col + 1) * width / grid_items;
            let y0 = row * height / grid_items;
            let y1 = (row + 1) * height / grid_items;
            let (tile_width, tile_height) = (x1 - x0, y1 - y0);

            let tile = imageops::crop_imm(&img, x0, y0, tile_width, tile_height);
            let avg = average_color(&tile.to_image());

            let key = runtime_cache
                .entry(avg)
                .or_insert_with(|| find_closest_match(cache, avg).to_owned())
                .clone();

            let decoded = image_cache
                .entry(key.clone())
                .or_insert_with(|| load_rgba(Path::new(&key)).expect("cache image disappeared"));

            let scaled = scaled_cache
                .entry((key.clone(), tile_width, tile_height))
                .or_insert_with(|| {
                    imageops::resize(
                        decoded,
                        tile_width,
                        tile_height,
                        imageops::FilterType::Triangle,
                    )
                });

            imageops::overlay(&mut canvas, scaled, x0.into(), y0.into());
            pb.inc(1);
        }
    }
    pb.finish_with_message("done");
    canvas.save(output)?;
    Ok(())
}
