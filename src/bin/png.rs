use anyhow::{Context, Result};
use idek_basics::Array2D;
use nalgebra::Vector2;
use slime::xiaolin;
use slime::{record::RecordFile, xiaolin::draw_line};
use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

type Rgb = [f32; 3];

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt()]
    record: PathBuf,

    #[structopt(short, long, default_value = "out.png")]
    outfile: PathBuf,

    #[structopt(short, long, default_value = "0")]
    first_frame: usize,

    #[structopt(short, long)]
    last_frame: Option<usize>,

    #[structopt(short, long, default_value = "1")]
    frame_step: usize,

    #[structopt(short, long, default_value = "0.01")]
    stroke_width: f32,

    #[structopt(short, long, default_value = "1000")]
    width: usize,

    #[structopt(short, long, default_value = "1000")]
    height: usize,

    /// Intensity of plotted points
    #[structopt(short, long, default_value = "0.01")]
    intensity: f32,
}

fn main() -> Result<()> {
    let args = Opt::from_args();

    let mut image: Array2D<[f32; 3]> = Array2D::new(args.width, args.height);

    println!("Loading...");
    let record = RecordFile::load(&args.record)?;

    let n_frames = record.frames.len();
    let last_frame = args.last_frame.unwrap_or(n_frames);

    let frames = &record.frames[args.first_frame..last_frame];
    let first = record.frames.first().context("No frames :/")?;

    let mut last = first;

    // Mapping from slime space to PNG space
    let coord_map = |v: Vector2<f32>| {
        (
            v.x * args.width as f32 / record.width as f32,
            v.y * args.height as f32 / record.height as f32,
        )
    };

    // Bounds check before plotting to image (additive)
    let mut plot_point = |x: i32, y: i32, color: [f32; 3]| {
        if x >= 0 && y >= 0 && x < args.width as i32 && y < args.height as i32 {
            image[(x as usize, y as usize)]
                .iter_mut()
                .zip(color)
                .for_each(|(o, i)| *o += i * args.intensity);
        }
    };

    println!("Building SVG...");
    for (idx, frame) in frames.into_iter().enumerate() {
        if idx % 100 == 0 {
            println!("{}/{}", idx, n_frames);
        }

        if idx % args.frame_step != 0 {
            continue;
        }

        for (part, prev) in frame.slime.iter().zip(&last.slime) {
            if part.age != 0 {
                let (x0, y0) = coord_map(prev.position);
                let (x1, y1) = coord_map(part.position);
                draw_line(x0, y0, x1, y1, |x, y, b| plot_point(x, y, [b; 3]));
            }
        }

        last = frame;
    }

    println!("Writing...");
    let data = rgb8_image(&image);
    write_png(&args.outfile, &data, args.width as _, args.height as _)?;

    Ok(())
}

/// Convert the given floating point image data to RGB8
fn rgb8_image(image: &Array2D<Rgb>) -> Vec<u8> {
    image
        .data()
        .iter()
        .map(|rgb| rgb.map(|x| (x.clamp(0., 1.) * 256.) as u8))
        .flatten()
        .collect()
}

/// Write a grayscale PNG at the given path
fn write_png(path: &Path, data: &[u8], width: u32, height: u32) -> Result<()> {
    let file = File::create(path)?;
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    writer.write_image_data(&data)?;

    Ok(())
}
