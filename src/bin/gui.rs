use idek::prelude::*;
use idek::winit;
use idek_basics::Array2D;
use idek_basics::{
    draw_array2d::draw_grid,
    idek::{self, simple_ortho_cam_ctx},
    GraphicsBuilder,
};
use nalgebra::Vector2;
use slime::{
    record::{record_frame, RecordFile},
    sim::*,
};
use std::path::Path;
use std::path::PathBuf;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = SlimeArgs::from_args();
    launch::<SlimeArgs, SlimeApp>(Settings::default().vr(args.vr).args(args))
}

#[derive(Clone, Default, Debug, StructOpt)]
struct SlimeArgs {
    #[structopt(short = "t", long, default_value = "0.5")]
    dt: f32,

    #[structopt(short = "w", long, default_value = "200")]
    width: usize,

    #[structopt(short = "h", long, default_value = "200")]
    height: usize,

    #[structopt(short = "n", long, default_value = "25000")]
    n_particles: usize,

    #[structopt(long)]
    vr: bool,

    #[structopt(long)]
    record: Option<PathBuf>,

    #[structopt(long, default_value = "1")]
    steps_per_frame: usize,

    #[structopt(long)]
    img: Option<PathBuf>,

    #[structopt(short = "l", long)]
    show_slime: bool,

    #[structopt(flatten)]
    cfg: SlimeConfig,
}

struct SlimeApp {
    verts: VertexBuffer,
    indices: IndexBuffer,
    args: SlimeArgs,
    sim: SlimeSim,
    gb: GraphicsBuilder,
    record: Option<RecordFile>,
    frame: usize,
}

impl App<SlimeArgs> for SlimeApp {
    fn init(ctx: &mut Context, _: &mut Platform, args: SlimeArgs) -> Result<Self> {
        let sim = SlimeSim::new(
            args.width,
            args.height,
            args.n_particles,
            &mut rand::thread_rng(),
        );

        let record = args
            .record
            .is_some()
            .then(|| RecordFile::new(args.width, args.height));

        let mut gb = GraphicsBuilder::new();

        draw_sim(&mut gb, &sim, &args);

        let verts = ctx.vertices(&gb.vertices, true)?;
        let indices = ctx.indices(&gb.indices, false)?;

        Ok(Self {
            frame: 0,
            record,
            verts,
            indices,
            gb,
            sim,
            args,
        })
    }

    fn frame(&mut self, ctx: &mut Context, platform: &mut Platform) -> Result<Vec<DrawCmd>> {
        // Timing
        for _ in 0..self.args.steps_per_frame {
            if let Some(record) = &mut self.record {
                record_frame(record, &mut self.sim);
            }

            self.sim
                .step(&self.args.cfg, self.args.dt, &mut rand::thread_rng());
        }

        // Update view
        self.gb.clear();

        if let Some(base_path) = self.args.img.as_ref() {
            let name = format!("{:04}.png", self.frame);
            let path = base_path.join(name);
            write_sim_frame(&path, self.sim.frame())?;
        }

        draw_sim(&mut self.gb, &self.sim, &self.args);
        ctx.update_vertices(self.verts, &self.gb.vertices)?;

        // Camera and drawing
        simple_ortho_cam_ctx(ctx, platform);

        self.frame += 1;

        Ok(vec![DrawCmd::new(self.verts).indices(self.indices)])
    }

    /// Called once per event
    fn event(&mut self, _ctx: &mut Context, platform: &mut Platform, event: Event) -> Result<()> {
        match (event, platform) {
            (
                Event::Winit(winit::event::Event::WindowEvent {
                    event: winit::event::WindowEvent::CloseRequested,
                    ..
                }),
                Platform::Winit { control_flow, .. },
            ) => {
                **control_flow = winit::event_loop::ControlFlow::Exit;
                self.exit();
            }
            _ => (),
        }
        Ok(())
    }
}

impl SlimeApp {
    fn exit(&self) {
        if let Some((record, path)) = self.record.as_ref().zip(self.args.record.as_ref()) {
            record.save(&path).expect("Failed to save");
        }
    }
}

fn color(v: f32) -> [f32; 3] {
    let v = v.clamp(0., 1.3);

    let k = v - 0.276;
    [(0.3, 0.8), (0.8, 1.0), (1.0, 0.678)]
        .map(|(a, b)| a * (1. - k) + b * k)
        .map(|c| c * v)
}

fn draw_sim(gb: &mut GraphicsBuilder, sim: &SlimeSim, cfg: &SlimeArgs) {
    let (slime, medium) = sim.frame();
    draw_grid(gb, medium, |&v| color(v), 0.5);
    if cfg.show_slime {
        draw_particles(gb, medium.width(), slime, 0., 0.002, [0.8, 0.0, 0.0]);
    }
}

fn draw_particles(
    gb: &mut GraphicsBuilder,
    width: usize,
    sim: &SlimeData,
    z: f32,
    scale: f32,
    color: [f32; 3],
) {
    for part in &sim.slime {
        let pos = part.position / width as f32;
        let pos = pos * 2. - Vector2::new(1., 1.);

        let mut push =
            |x: f32, y: f32| gb.push_vertex(Vertex::new([x + pos.x, y + pos.y, z], color));

        let tl = push(-scale, -scale);
        let tr = push(scale, -scale);

        let bl = push(-scale, scale);
        let br = push(scale, scale);

        gb.push_indices(&[bl, tr, tl, bl, br, tr]);
        gb.push_indices(&[tl, tr, bl, tr, br, bl]);
    }
}

fn write_sim_frame(path: &Path, (_slime, medium): (&SlimeData, &Array2D<f32>)) -> Result<()> {
    let val_to_color = |v: f32| color(v).map(|c| (c.sqrt().clamp(0., 1.) * 256.) as u8);

    let data: Vec<u8> = medium
        .data()
        .iter()
        .copied()
        .map(val_to_color)
        .flatten()
        .collect();

    //let mut image_u8 = Array2D::from_array(medium.width(), data);

    /*
    for particle in slime.slime {
        particle.position
    }
    */

    // For reading and opening files
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(path)?;
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, medium.width() as _, medium.height() as _); // Width is 2 pixels and height is 1.
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    writer.write_image_data(&data)?;

    Ok(())
}
