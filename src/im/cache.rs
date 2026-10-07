use std::collections::HashMap;
use std::path::Path;

use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use serde::{Deserialize, Serialize};

use super::color::Rgb;
use super::image::{compute_average_color, get_image_paths, load_rgb};

const IMAGE_DB_FILE_NAME: &str = "mosaic.cache.json";

pub struct Cache {
    color_cache: HashMap<String, Rgb>,
}

#[derive(Serialize, Deserialize)]
struct SerializableCache {
    entries: HashMap<String, Rgb>,
}

impl Cache {
    pub const fn new(color_cache: HashMap<String, Rgb>) -> Self {
        Self { color_cache }
    }

    pub fn is_empty(&self) -> bool {
        self.color_cache.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Rgb)> {
        self.color_cache.iter()
    }

    pub fn generate_from_path(src: &Path, rebuild: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let src = std::fs::canonicalize(src).map_err(|e| format!("cannot resolve source directory: {e}"))?;
        let cache_file_location = src.join(IMAGE_DB_FILE_NAME);

        if cache_file_location.exists() && !rebuild {
            let content = std::fs::read_to_string(&cache_file_location).map_err(|e| format!("cannot read cache file: {e}"))?;
            let serializable: SerializableCache = serde_json::from_str(&content).map_err(|e| format!("cache file is corrupt: {e}"))?;

            let color_cache: HashMap<String, Rgb> = serializable
                .entries
                .into_iter()
                .map(|(rel, avg)| {
                    let full_path = src.join(&rel).to_string_lossy().into_owned();
                    (full_path, avg)
                })
                .collect();

            println!("Read {} items from existing cache file.", color_cache.len());
            return Ok(Self::new(color_cache));
        }

        let paths = get_image_paths(&src);

        if paths.is_empty() {
            return Err("Could not find any images for creating the cache. Exiting.".into());
        }

        let cache_data: Vec<(String, Rgb)> = paths
            .into_par_iter()
            .filter_map(|path| {
                let img = load_rgb(&path)?;
                let path_str = path.to_string_lossy().into_owned();
                let avg = compute_average_color(&img);
                Some((path_str, avg))
            })
            .collect();

        if cache_data.is_empty() {
            return Err("Could not find any images for creating the cache. Exiting.".into());
        }

        let color_cache: HashMap<String, Rgb> = cache_data.clone().into_iter().collect();

        let rel_cache = cache_data
            .into_iter()
            .map(|(path, avg)| {
                let p = Path::new(&path);
                let rel = p.strip_prefix(&src).unwrap_or(p).to_string_lossy().into_owned();
                (rel, avg)
            })
            .collect::<HashMap<_, _>>();

        let serializable = SerializableCache { entries: rel_cache };
        let json = serde_json::to_string_pretty(&serializable)?;
        std::fs::write(&cache_file_location, json)?;

        println!("Created cache file \"{}\" with {} entries.", cache_file_location.display(), color_cache.len());

        Ok(Self::new(color_cache))
    }

    pub fn find_closest_match(&self, target: Rgb) -> &str {
        assert!(!self.is_empty());
        self.iter()
            .min_by_key(|(_, color)| super::color::rgb_squared_distance(target, **color))
            .map(|(k, _)| k.as_str())
            .expect("cache must not be empty")
    }
}
