use anyhow::{Context, Result};
use idek_basics::Array2D;
use slime::{record::RecordFile, xiaolin::draw_line};
use std::{path::{PathBuf, Path}, fs::File, io::BufWriter};
use structopt::StructOpt;
use slime::xiaolin;

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
}

fn main() -> Result<()> {
    let args = Opt::from_args();

    let mut image = Array2D::new(args.width, args.height);

    let n = 365;
    for i in 0..n {
        let i = i as f32 / n as f32;

        let i = i * std::f32::consts::TAU;

        let x0 = args.width as f32 / 2.;
        let y0 = args.height as f32 / 2.;

        let r = args.width as f32 / 3.;

        let x1 = r * i.cos() + x0;
        let y1 = r * i.sin() + y0;

        draw_line(x0, y0, x1, y1, |x, y, b| image[(x as usize, y as usize)] = [b; 3]);
    }

    /*
    println!("Loading...");
    let record = RecordFile::load(&args.record)?;

    let n_frames = record.frames.len();
    let last_frame = args.last_frame.unwrap_or(n_frames);

    let frames = &record.frames[args.first_frame..last_frame];
    let first = record.frames.first().context("No frames :/")?;

    let mut last = first;

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
                todo!()
            }
        }

        last = frame;
    }
    */

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


