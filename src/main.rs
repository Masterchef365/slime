use idek::prelude::*;
use idek_basics::{
    draw_array2d::draw_grid,
    idek::{self, simple_ortho_cam_ctx},
    GraphicsBuilder,
};

use slime::*;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = SlimeArgs::from_args();
    launch::<SlimeArgs, SlimeApp>(Settings::default().vr(args.vr).args(args))
}

#[derive(Clone, Default, Debug, StructOpt)]
struct SlimeArgs {
    #[structopt(short = "t", long, default_value = "0.5")]
    dt: f32,

    #[structopt(short = "w", long, default_value = "400")]
    width: usize,

    #[structopt(short = "h", long, default_value = "400")]
    height: usize,

    #[structopt(short = "n", long, default_value = "4000")]
    n_particles: usize,

    #[structopt(long)]
    vr: bool,

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
}

impl App<SlimeArgs> for SlimeApp {
    fn init(ctx: &mut Context, _: &mut Platform, args: SlimeArgs) -> Result<Self> {
        let sim = SlimeSim::new(
            args.width,
            args.height,
            args.n_particles,
            &mut rand::thread_rng(),
        );

        let mut gb = GraphicsBuilder::new();

        draw_sim(&mut gb, &sim);

        let verts = ctx.vertices(&gb.vertices, true)?;
        let indices = ctx.indices(&gb.indices, false)?;

        Ok(Self {
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
            self.sim
                .step(&self.args.cfg, self.args.dt, &mut rand::thread_rng());
        }

        // Update view
        self.gb.clear();
        draw_sim(&mut self.gb, &self.sim);
        ctx.update_vertices(self.verts, &self.gb.vertices)?;

        // Camera and drawing
        simple_ortho_cam_ctx(ctx, platform);

        Ok(vec![DrawCmd::new(self.verts).indices(self.indices)])
    }
}

fn draw_sim(gb: &mut GraphicsBuilder, sim: &SlimeSim) {
    draw_grid(gb, &sim.frame().medium, |&v| [v; 3], 0.);
}
