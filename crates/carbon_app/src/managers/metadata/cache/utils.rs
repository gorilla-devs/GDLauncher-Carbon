// Utility functions for cache operations

use anyhow::Result;
use image::ImageFormat;

/// Scale a mod image to 256x256 pixels
pub fn scale_mod_image(input_bytes: &[u8]) -> Result<Vec<u8>> {
    let input_image = image::io::Reader::new(std::io::Cursor::new(input_bytes))
        .with_guessed_format()?
        .decode()?;

    let output_image = input_image.resize(256, 256, image::imageops::FilterType::CatmullRom);

    let mut output_bytes = std::io::Cursor::new(Vec::new());
    output_image.write_to(&mut output_bytes, ImageFormat::Png)?;

    Ok(output_bytes.into_inner())
}
