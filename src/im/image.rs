use std::path::{Path, PathBuf};

use image::{ImageReader, RgbImage, imageops};

use super::color::Rgb;

const AVG_THUMBNAIL_SIZE: u32 = 256;

const SUPPORTED_FILE_TYPES: [&str; 5] = ["bmp", "jpeg", "jpg", "png", "webp"];

fn is_supported_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SUPPORTED_FILE_TYPES.contains(&ext.to_ascii_lowercase().as_str()))
}

#[cfg(unix)]
fn is_hidden(entry: &std::fs::DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(|s| s.starts_with('.'))
}

fn collect_paths_from_dir(dir: &Path, paths: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            #[cfg(unix)]
            if is_hidden(&entry) {
                continue;
            }

            let path = entry.path();
            if path.is_dir() {
                collect_paths_from_dir(path.as_path(), paths);
            } else if is_supported_file(path.as_path()) {
                paths.push(path);
            }
        }
    }
}

pub fn get_image_paths(src: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_paths_from_dir(src, &mut paths);
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    paths
}

pub fn load_rgb(path: &Path) -> Option<RgbImage> {
    ImageReader::open(path)
        .and_then(|reader| reader.decode().map_err(std::io::Error::other))
        .map_err(|e| eprintln!("skipping {}: {e}", path.display()))
        .ok()
        .map(|img| img.to_rgb8())
}

pub fn compute_average_color(img: &RgbImage) -> Rgb {
    let thumb = imageops::thumbnail(img, AVG_THUMBNAIL_SIZE, AVG_THUMBNAIL_SIZE);
    super::color::average_color_rgb(&thumb)
}

pub fn resize_image(img: &RgbImage, width: u32, height: u32) -> RgbImage {
    imageops::resize(img, width, height, imageops::FilterType::Nearest)
}
