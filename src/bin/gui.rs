use idek::prelude::*;
use idek::winit;
use idek_basics::{
    draw_array2d::draw_grid,
    idek::{self, simple_ortho_cam_ctx},
    GraphicsBuilder,
};
use slime::{
    record::{record_frame, RecordFile},
    sim::*,
};
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

    #[structopt(short = "w", long, default_value = "60")]
    width: usize,

    #[structopt(short = "h", long, default_value = "60")]
    height: usize,

    #[structopt(short = "h", long, default_value = "60")]
    depth: usize,

    #[structopt(short = "n", long, default_value = "4000")]
    n_particles: usize,

    #[structopt(long)]
    vr: bool,

    #[structopt(long)]
    record: Option<PathBuf>,

    #[structopt(long, default_value = "1")]
    steps_per_frame: usize,

    #[structopt(flatten)]
    cfg: SlimeConfig,
}

struct SlimeApp {
    verts: VertexBuffer,
    indices: IndexBuffer,
    args: SlimeArgs,
    sim: SlimeSim,
    gb: GraphicsBuilder,
    record: RecordFile,
    camera: MultiPlatformCamera,
}

impl App<SlimeArgs> for SlimeApp {
    fn init(ctx: &mut Context, platform: &mut Platform, args: SlimeArgs) -> Result<Self> {
        let sim = SlimeSim::new(
            args.width,
            args.height,
            args.depth,
            args.n_particles,
            &mut rand::thread_rng(),
        );

        let record = RecordFile::new(args.width, args.height);

        let mut gb = GraphicsBuilder::new();

        draw_sim(&mut gb, &sim);

        let verts = ctx.vertices(&gb.vertices, true)?;
        let indices = ctx.indices(&gb.indices, false)?;

        Ok(Self {
            camera: MultiPlatformCamera::new(platform),
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
            record_frame(&mut self.record, &mut self.sim);
            self.sim
                .step(&self.args.cfg, self.args.dt, &mut rand::thread_rng());
        }

        // Update view
        self.gb.clear();
        draw_sim(&mut self.gb, &self.sim);
        ctx.update_vertices(self.verts, &self.gb.vertices)?;

        Ok(vec![DrawCmd::new(self.verts).indices(self.indices)])
    }

    /// Called once per event
    fn event(&mut self, ctx: &mut Context, platform: &mut Platform, mut event: Event) -> Result<()> {
        if self.camera.handle_event(&mut event) {
            ctx.set_camera_prefix(self.camera.get_prefix())
        }

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
        if let Some(path) = self.args.record.as_ref() {
            self.record.save(&path).expect("Failed to save");
        }
    }
}

fn draw_sim(gb: &mut GraphicsBuilder, sim: &SlimeSim) {
    let base = gb.push_vertex(Vertex::new([-1.0, -1.0, -1.0], [0.0, 1.0, 1.0]));
    gb.push_vertex(Vertex::new([1.0, -1.0, -1.0], [1.0, 0.0, 1.0]));
    gb.push_vertex(Vertex::new([1.0, 1.0, -1.0], [1.0, 1.0, 0.0]));
    gb.push_vertex(Vertex::new([-1.0, 1.0, -1.0], [0.0, 1.0, 1.0]));
    gb.push_vertex(Vertex::new([-1.0, -1.0, 1.0], [1.0, 0.0, 1.0]));
    gb.push_vertex(Vertex::new([1.0, -1.0, 1.0], [1.0, 1.0, 0.0]));
    gb.push_vertex(Vertex::new([1.0, 1.0, 1.0], [0.0, 1.0, 1.0]));
    gb.push_vertex(Vertex::new([-1.0, 1.0, 1.0], [1.0, 0.0, 1.0]));

    let indices = [
        3, 1, 0, 2, 1, 3, 2, 5, 1, 6, 5, 2, 6, 4, 5, 7, 4, 6, 7, 0, 4, 3, 0, 7, 7, 2, 3, 6, 2, 7,
        0, 5, 4, 1, 5, 0,
    ];

    gb.push_indices(&indices.map(|i| i + base));

    //draw_grid(gb, &sim.frame().medium, |&v| [v; 3], 0.);
}
