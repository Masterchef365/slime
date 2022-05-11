use anyhow::{Context, Result};
use slime::record::RecordFile;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt()]
    record: PathBuf,

    #[structopt(short, long, default_value = "out.svg")]
    outfile: PathBuf,

    #[structopt(short, long, default_value = "0")]
    first_frame: usize,

    #[structopt(short, long)]
    last_frame: Option<usize>,

    #[structopt(short, long, default_value = "1")]
    frame_step: usize,

    #[structopt(short, long, default_value = "0.01")]
    stroke_width: f32,
}
use svg::node::element::path::Data;
use svg::node::element::Path;
use svg::Document;

fn main() -> Result<()> {
    let args = Opt::from_args();

    let mut document = Document::new().set("viewBox", (0, 0, 400, 400));

    println!("Loading...");
    let record = RecordFile::load(&args.record)?;

    let n_frames = record.frames.len();

    let last_frame = args.last_frame.unwrap_or(n_frames);

    let frames = &record.frames[args.first_frame..last_frame];

    let first = record.frames.first().context("No frames :/")?;

    let mut paths: Vec<Option<Data>> = vec![None; first.slime.len()];

    let finish_path = |data| {
        Path::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", args.stroke_width)
            .set("d", data)
    };

    println!("Building SVG...");
    for (idx, frame) in frames.into_iter().enumerate() {
        if idx % 100 == 0 {
            println!("{}/{}", idx, n_frames);
        }

        for (part, path) in frame.slime.iter().zip(&mut paths) {
            if part.age == 0 {
                let new_path = Data::new().move_to((part.position.x, part.position.y));

                if let Some(finished) = path.replace(new_path) {
                    document = document.add(finish_path(finished));
                }
            } else {
                if idx % args.frame_step == 0 {
                    let line = path
                        .take()
                        .map(|path| path.line_to((part.position.x, part.position.y)));
                    *path = line;
                }
            }
        }
    }

    println!("Finishing paths...");
    for path in paths {
        if let Some(path) = path {
            document = document.add(finish_path(path));
        }
    }

    println!("Writing...");
    svg::save(args.outfile, &document)?;

    Ok(())
}
