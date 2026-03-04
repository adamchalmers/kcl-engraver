use std::error::Error;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use clap::Parser;
use image::codecs::png::PngEncoder;
use image::{ColorType, DynamicImage, GrayImage, ImageEncoder};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = parse_args();
    let input = load_input_image(&args.input)?;
    let output = engrave(input, args.resolution);
    write_output_image(&output, &args.output)?;
    Ok(())
}

#[derive(Parser, Debug, Clone)]
#[command(name = "kcl-engraver")]
struct Args {
    /// Input PNG path, or '-' to read PNG bytes from stdin.
    #[arg(value_name = "INPUT")]
    input: String,
    /// Output PNG path, or '-' to write PNG bytes to stdout.
    #[arg(value_name = "OUTPUT", default_value = "out.png")]
    output: String,
    /// Block size in pixels (must be >= 1).
    #[arg(short, long, value_name = "N", value_parser = clap::value_parser!(u32).range(1..))]
    resolution: u32,
}

fn parse_args() -> Args {
    Args::parse()
}

fn load_input_image(input: &str) -> Result<DynamicImage, Box<dyn Error>> {
    if input == "-" {
        let mut bytes = Vec::new();
        io::stdin().read_to_end(&mut bytes)?;
        if bytes.is_empty() {
            return Err("stdin was empty".into());
        }
        Ok(image::load_from_memory(&bytes)?)
    } else {
        Ok(image::open(Path::new(input))?)
    }
}

fn write_output_image(image: &GrayImage, output: &str) -> Result<(), Box<dyn Error>> {
    let (width, height) = image.dimensions();
    if output == "-" {
        let mut stdout = io::stdout().lock();
        let encoder = PngEncoder::new(&mut stdout);
        encoder.write_image(image.as_raw(), width, height, ColorType::L8.into())?;
    } else {
        let file = File::create(Path::new(output))?;
        let encoder = PngEncoder::new(file);
        encoder.write_image(image.as_raw(), width, height, ColorType::L8.into())?;
    }
    Ok(())
}

fn engrave(input: DynamicImage, resolution: u32) -> GrayImage {
    let grayscale = input.to_luma8();
    let (width, height) = grayscale.dimensions();

    let grid_width = width.div_ceil(resolution);
    let grid_height = height.div_ceil(resolution);

    let mut levels = vec![0.0_f32; (grid_width * grid_height) as usize];
    for gy in 0..grid_height {
        for gx in 0..grid_width {
            let start_x = gx * resolution;
            let start_y = gy * resolution;
            let end_x = (start_x + resolution).min(width);
            let end_y = (start_y + resolution).min(height);

            let mut total = 0_u64;
            let mut count = 0_u64;
            for y in start_y..end_y {
                for x in start_x..end_x {
                    total += grayscale.get_pixel(x, y).0[0] as u64;
                    count += 1;
                }
            }

            let idx = (gy * grid_width + gx) as usize;
            levels[idx] = (total as f32) / (count as f32);
        }
    }

    let mut bw = vec![0_u8; (grid_width * grid_height) as usize];
    for gy in 0..grid_height {
        for gx in 0..grid_width {
            let idx = (gy * grid_width + gx) as usize;
            let old = levels[idx];
            let new = if old < 128.0 { 0.0 } else { 255.0 };
            bw[idx] = new as u8;
            levels[idx] = new;

            let err = old - new;
            diffuse_error(
                &mut levels,
                grid_width,
                grid_height,
                gx + 1,
                gy,
                err * 7.0 / 16.0,
            );
            if gx > 0 {
                diffuse_error(
                    &mut levels,
                    grid_width,
                    grid_height,
                    gx - 1,
                    gy + 1,
                    err * 3.0 / 16.0,
                );
            }
            diffuse_error(
                &mut levels,
                grid_width,
                grid_height,
                gx,
                gy + 1,
                err * 5.0 / 16.0,
            );
            diffuse_error(
                &mut levels,
                grid_width,
                grid_height,
                gx + 1,
                gy + 1,
                err * 1.0 / 16.0,
            );
        }
    }

    let mut output = GrayImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let gx = x / resolution;
            let gy = y / resolution;
            let value = bw[(gy * grid_width + gx) as usize];
            output.get_pixel_mut(x, y).0[0] = value;
        }
    }

    output
}

fn diffuse_error(
    levels: &mut [f32],
    grid_width: u32,
    grid_height: u32,
    x: u32,
    y: u32,
    delta: f32,
) {
    if x >= grid_width || y >= grid_height {
        return;
    }
    let idx = (y * grid_width + x) as usize;
    levels[idx] = (levels[idx] + delta).clamp(0.0, 255.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Luma};

    #[test]
    fn preserves_dimensions() {
        let input = DynamicImage::ImageLuma8(ImageBuffer::from_pixel(9, 7, Luma([128])));
        let out = engrave(input, 4);
        assert_eq!(out.dimensions(), (9, 7));
    }

    #[test]
    fn output_is_binary() {
        let input = DynamicImage::ImageLuma8(ImageBuffer::from_fn(8, 8, |x, _| {
            if x < 4 { Luma([20]) } else { Luma([220]) }
        }));
        let out = engrave(input, 2);
        for p in out.pixels() {
            assert!(p.0[0] == 0 || p.0[0] == 255);
        }
    }
}
