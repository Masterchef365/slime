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
    args: SlimeArgs,
    sim: SlimeSim,
    record: RecordFile,

    gb: GraphicsBuilder,
    verts: VertexBuffer,
    indices: IndexBuffer,
    shader: Shader,

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
        let shader = ctx.shader(
            DEFAULT_VERTEX_SHADER,
            DEFAULT_FRAGMENT_SHADER,
            Primitive::Points,
        )?;

        Ok(Self {
            camera: MultiPlatformCamera::new(platform),
            shader,
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

        Ok(vec![DrawCmd::new(self.verts)
            .indices(self.indices)
            .shader(self.shader)])
    }

    /// Called once per event
    fn event(
        &mut self,
        ctx: &mut Context,
        platform: &mut Platform,
        mut event: Event,
    ) -> Result<()> {
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
    let scale = 5.;
    let color = [1.; 3];

    let frame = sim.frame();
    let medium = &frame.medium;

    let n_width_verts = medium.width() + 1;
    let n_height_verts = medium.height() + 1;
    let n_depth_verts = medium.length() + 1;

    let map = |v| (v * 2. - 1.) * scale;

    let base = gb.indices.len() as u32;

    for k in 0..n_depth_verts {
        let z = map(k as f32 / n_depth_verts as f32);
        for j in 0..n_height_verts {
            let y = map(j as f32 / n_height_verts as f32);
            for i in 0..n_width_verts {
                let x = map(i as f32 / n_width_verts as f32);

                if i < medium.width() && j < medium.height() && k < medium.length() {
                    let v = medium[(i, j, k)];

                    let color = [v * 2.; 3];

                    let idx = gb.push_vertex(Vertex::new([x, y, z], color));
                    gb.push_indices(&[idx]);
                }

            }
        }
    }

    //gb.push_indices(&indices.map(|i| i + base));

    //draw_grid(gb, &sim.frame().medium, |&v| [v; 3], 0.);
}
