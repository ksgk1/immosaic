use std::collections::{HashMap, HashSet};
use std::path::Path;

use image::{RgbImage, imageops};
use rayon::iter::{IntoParallelIterator as _, IntoParallelRefIterator as _, ParallelIterator as _};

use super::cache::Cache;
use super::color::Rgb;
use super::image::{load_rgb, resize_image};
use crate::progress::ProgressBar;

/// Key for looking up scaled images: (source image key, width, height)
type ImageKey = (String, u32, u32);

/// Cache of loaded and resized images, keyed by source image and dimensions
type ScaledImageCache = HashMap<ImageKey, RgbImage>;

struct RawTile {
    x:      i64,
    y:      i64,
    width:  u32,
    height: u32,
    color:  Rgb,
}

struct Tile {
    x:      i64,
    y:      i64,
    width:  u32,
    height: u32,
    key:    String,
}

impl Cache {
    pub fn generate_mosaic(&self, input_image: &Path, output: &Path, grid_items: u32, scale: u32) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_empty() {
            return Err("Cannot create image without cache.".into());
        }

        let grid_items = grid_items.max(1);
        let scale = scale.max(1);

        let img = load_rgb(input_image).ok_or_else(|| format!("failed to load image: {}", input_image.display()))?;
        let (width, height) = img.dimensions();
        let (out_width, out_height) = (width * scale, height * scale);

        let tiles = Self::compute_tiles(&img, width, height, grid_items);
        let tiles = self.resolve_tiles_to_images(tiles);
        let scaled = Self::load_and_resize_source_images(&tiles, scale);
        Self::assemble_mosaic(&tiles, &scaled, out_width, out_height, output, scale)?;

        Ok(())
    }

    fn compute_tiles(img: &RgbImage, width: u32, height: u32, grid_items: u32) -> Vec<RawTile> {
        (0..grid_items)
            .into_par_iter()
            .flat_map(|row| (0..grid_items).into_par_iter().map(move |col| (row, col)))
            .map(|(row, col)| {
                let x0 = col * width / grid_items;
                let x1 = (col + 1) * width / grid_items;
                let y0 = row * height / grid_items;
                let y1 = (row + 1) * height / grid_items;

                let tile_width = x1 - x0;
                let tile_height = y1 - y0;
                let tile = imageops::crop_imm(img, x0, y0, tile_width, tile_height);
                let avg = super::color::average_color_rgb(&tile.to_image());

                RawTile {
                    x:      i64::from(x0),
                    y:      i64::from(y0),
                    width:  tile_width,
                    height: tile_height,
                    color:  avg,
                }
            })
            .collect()
    }

    fn resolve_tiles_to_images(&self, tiles: Vec<RawTile>) -> Vec<Tile> {
        let mut runtime_color_cache = HashMap::<Rgb, String>::new();

        tiles
            .into_iter()
            .map(|tile| {
                let key = runtime_color_cache
                    .entry(tile.color)
                    .or_insert_with(|| self.find_closest_match(tile.color).into());
                Tile {
                    x:      tile.x,
                    y:      tile.y,
                    width:  tile.width,
                    height: tile.height,
                    key:    key.clone(),
                }
            })
            .collect()
    }

    fn load_and_resize_source_images(tiles: &[Tile], scale: u32) -> ScaledImageCache {
        let wanted: HashSet<ImageKey> = tiles.par_iter().map(|t| (t.key.clone(), t.width * scale, t.height * scale)).collect();

        let pb_load = ProgressBar::with_prefix(wanted.len() as u64, "Loading data");

        let scaled = wanted
            .into_par_iter()
            .filter_map(|(key, tile_width, tile_height)| {
                let img = load_rgb(Path::new(&key))?;
                let resized = resize_image(&img, tile_width, tile_height);
                pb_load.inc(1);
                Some(((key, tile_width, tile_height), resized))
            })
            .collect();

        pb_load.finish();
        scaled
    }

    fn assemble_mosaic(
        tiles: &[Tile], scaled: &ScaledImageCache, out_width: u32, out_height: u32, output: &Path, scale: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pb_scale = ProgressBar::with_prefix(tiles.len() as u64, "Scaling");

        let scaled_tiles: Vec<(&RgbImage, i64, i64)> = tiles
            .par_iter()
            .filter_map(|tile| {
                let key: ImageKey = (tile.key.clone(), tile.width * scale, tile.height * scale);
                let s = scaled.get(&key)?;
                pb_scale.inc(1);
                Some((s, tile.x * i64::from(scale), tile.y * i64::from(scale)))
            })
            .collect();

        pb_scale.finish();

        let pb_gen = ProgressBar::with_prefix(tiles.len() as u64, "Generating");

        let mut canvas = RgbImage::new(out_width, out_height);
        for (s, x, y) in scaled_tiles {
            imageops::overlay(&mut canvas, s, x, y);
            pb_gen.inc(1);
        }

        pb_gen.finish();
        canvas.save(output)?;
        Ok(())
    }
}
