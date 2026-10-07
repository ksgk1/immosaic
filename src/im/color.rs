use rayon::iter::ParallelIterator as _;

pub type Rgb = [u8; 3];

#[inline]
pub fn rgb_squared_distance(lhs: Rgb, rhs: Rgb) -> u32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(a, b)| {
            let d = i32::from(*a) - i32::from(*b);
            (d * d).cast_unsigned()
        })
        .sum()
}

#[inline]
pub fn average_color_rgb(img: &image::RgbImage) -> Rgb {
    let (width, height) = img.dimensions();
    let total_pixels = u64::from(width) * u64::from(height);

    if total_pixels == 0 {
        return [0, 0, 0];
    }

    let sums = img
        .par_pixels()
        .fold(
            || [0u64; 3],
            |mut acc, pixel| {
                acc[0] += u64::from(pixel[0]);
                acc[1] += u64::from(pixel[1]);
                acc[2] += u64::from(pixel[2]);
                acc
            },
        )
        .reduce(
            || [0u64; 3],
            |mut a, b| {
                a[0] += b[0];
                a[1] += b[1];
                a[2] += b[2];
                a
            },
        );

    [(sums[0] / total_pixels) as u8, (sums[1] / total_pixels) as u8, (sums[2] / total_pixels) as u8]
}
