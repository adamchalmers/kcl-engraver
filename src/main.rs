use std::error::Error;
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
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

    let kcl_output = args.kcl || args.output.ends_with(".kcl");
    if kcl_output {
        let coords = extract_black_block_coords(&output, args.resolution);
        write_kcl_output(&coords, &args.output)?;
    } else {
        write_output_image(&output, &args.output)?;
    }
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
    /// Emit KCL coordinate output even when OUTPUT is '-'.
    #[arg(long)]
    kcl: bool,
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

fn write_kcl_output(coords: &[(u32, u32)], output: &str) -> Result<(), Box<dyn Error>> {
    if output == "-" {
        let mut stdout = io::stdout().lock();
        write_kcl_coords(&mut stdout, coords)?;
    } else {
        let file = File::create(Path::new(output))?;
        let mut writer = BufWriter::new(file);
        write_kcl_coords(&mut writer, coords)?;
    }
    Ok(())
}

const PREAMBLE: &str = "
fn block(x, y) {
    return startSketchOn(XY)
    |> rectangle(width = 1, height = 1, corner = [x, y])
    |> extrude(length = 1)
}
";

fn write_kcl_coords<W: Write>(writer: &mut W, coords: &[(u32, u32)]) -> io::Result<()> {
    writeln!(writer, "{}", PREAMBLE)?;
    for (x, y) in coords {
        writeln!(writer, "block(x = {x}, y = {y})")?;
    }
    Ok(())
}

fn extract_black_block_coords(image: &GrayImage, resolution: u32) -> Vec<(u32, u32)> {
    let (width, height) = image.dimensions();
    let grid_width = width.div_ceil(resolution);
    let grid_height = height.div_ceil(resolution);

    let mut coords = Vec::new();
    for gy in 0..grid_height {
        for gx in 0..grid_width {
            let x = gx * resolution;
            let y = gy * resolution;
            if image.get_pixel(x, y).0[0] == 0 {
                coords.push((gx, gy));
            }
        }
    }
    coords
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

    #[test]
    fn extracts_black_block_coords_from_grid_pixels() {
        let mut image = GrayImage::from_pixel(4, 4, image::Luma([255]));
        for y in 0..2 {
            for x in 0..2 {
                image.get_pixel_mut(x, y).0[0] = 0;
            }
        }
        for y in 2..4 {
            for x in 2..4 {
                image.get_pixel_mut(x, y).0[0] = 0;
            }
        }

        let coords = extract_black_block_coords(&image, 2);
        assert_eq!(coords, vec![(0, 0), (1, 1)]);
    }
}
