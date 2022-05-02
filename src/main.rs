use idek::prelude::*;
use idek_basics::{
    draw_array2d::draw_grid_fuzzy,
    idek::{
        self,
        nalgebra::{Rotation2, Vector1, Vector2},
        simple_ortho_cam_ctx,
    },
    Array2D, GraphicsBuilder,
};
use rand::{distributions::Uniform, prelude::*};
use std::f32::consts::TAU;
use structopt::StructOpt;

fn main() -> Result<()> {
    let args = SlimeArgs::from_args();
    launch::<SlimeArgs, SlimeApp>(Settings::default().vr(args.vr).args(args))
}

#[derive(Clone, Default, Debug, StructOpt)]
struct SlimeArgs {
    #[structopt(short = "t", long, default_value = "0.1")]
    dt: f32,

    #[structopt(short = "w", long, default_value = "400")]
    width: usize,

    #[structopt(short = "h", long, default_value = "400")]
    height: usize,

    #[structopt(short = "n", long, default_value = "2000")]
    n_particles: usize,

    #[structopt(long)]
    vr: bool,

    #[structopt(flatten)]
    cfg: SlimeConfig,
}

#[derive(Clone, Default, Debug, StructOpt)]
struct SlimeConfig {
    /// Angle between adjacent sensors (radians)
    #[structopt(short = "s", long, default_value = "0.26")]
    sensor_spread: f32,

    /// Turn rate, radians/time
    #[structopt(short = "r", long, default_value = "0.26")]
    turn_speed: f32,

    /// Trail/slime decay rate
    #[structopt(short = "d", long, default_value = "0.01")]
    decay: f32,

    /// Deposit rate for slime
    #[structopt(short = "e", long, default_value = "0.1")]
    deposit_rate: f32,

    /// Slime movement speed
    #[structopt(short = "m", long, default_value = "1.")]
    move_speed: f32,

    /// Sample distance
    #[structopt(short = "u", long, default_value = "1.")]
    sample_dist: f32,

    /// Diffusion rate of the medium
    #[structopt(short = "i", long, default_value = "1.")]
    diffusion: f32,
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
        self.sim
            .step(&self.args.cfg, self.args.dt, &mut rand::thread_rng());

        // Update view
        self.gb.clear();
        draw_sim(&mut self.gb, &self.sim);
        ctx.update_vertices(self.verts, &self.gb.vertices)?;

        // Camera and drawing
        simple_ortho_cam_ctx(ctx, platform);

        Ok(vec![DrawCmd::new(self.verts).indices(self.indices)])
    }
}

#[derive(Clone, Copy)]
struct SlimeParticle {
    pub position: Vector2<f32>,
    pub heading: Vector2<f32>,
}

#[derive(Clone)]
struct SlimeData {
    medium: Array2D<f32>,
    slime: Vec<SlimeParticle>,
}

struct SlimeSim {
    /// The buffer to be presented to the user and read by the sim
    front: SlimeData,
    /// The buffer which is written to by the sim and swapped to front each frame
    back: SlimeData,
    /// Slime factory
    factory: SlimeFactory,
}

fn unit_circ(a: f32) -> Vector2<f32> {
    Vector2::new(a.cos(), a.sin())
}

impl SlimeSim {
    pub fn new(width: usize, height: usize, n_particles: usize, mut rng: impl Rng) -> Self {
        let factory = SlimeFactory::new(width, height);

        let slime = (0..n_particles).map(|_| factory.slime(&mut rng)).collect();

        let front = SlimeData {
            slime,
            medium: Array2D::new(width, height),
        };

        Self {
            back: front.clone(),
            front,
            factory,
        }
    }

    pub fn frame(&self) -> &SlimeData {
        &self.front
    }

    pub fn step(&mut self, cfg: &SlimeConfig, dt: f32, rng: impl Rng) {
        self.diffuse_step(cfg, dt);
        self.particle_step(cfg, dt, rng);
        std::mem::swap(&mut self.front, &mut self.back);
    }

    fn diffuse_step(&mut self, cfg: &SlimeConfig, dt: f32) {
        // Diffusion and decay
        for y in 0..self.front.medium.height() {
            for x in 0..self.front.medium.width() {
                let mut sum = 0.;
                let mut n_parts = 0;
                for i in -1..=1 {
                    for j in -1..=1 {
                        if let Some(v) =
                            sample_array_isize(&self.front.medium, j + x as isize, i + y as isize)
                        {
                            sum += v;
                            n_parts += 1;
                        }
                    }
                }

                let avg = sum / n_parts as f32;

                let pos = (x, y);
                let center = self.front.medium[pos];

                let diffuse = mix(center, avg, cfg.diffusion * dt);

                let decayed = (1. - cfg.decay * dt) * diffuse;

                self.back.medium[pos] = decayed;
            }
        }
    }

    fn particle_step(&mut self, cfg: &SlimeConfig, dt: f32, mut rng: impl Rng) {
        // Some premature optimization
        let left_sensor_rot = Rotation2::from_scaled_axis(Vector1::new(cfg.sensor_spread) * dt);
        let right_sensor_rot = left_sensor_rot.inverse();

        let left_turn_rate = Rotation2::from_scaled_axis(Vector1::new(cfg.turn_speed) * dt);
        let right_turn_rate = left_turn_rate.inverse();

        let unit_rot = Rotation2::identity();

        // Step particle motion
        for (b, f) in self.back.slime.iter_mut().zip(&self.front.slime) {
            // Sample the grid
            let [left, center, right] = [left_sensor_rot, unit_rot, right_sensor_rot]
                .map(|r| f.position + r * f.heading * cfg.sample_dist)
                .map(|p| sample_array_vect(&self.back.medium, p))
                .map(|p| p.map(|p| self.front.medium[p]));

            // Decide which way to go
            let lc = left.partial_cmp(&center);
            let cr = center.partial_cmp(&right);

            use std::cmp::Ordering as Odr;

            let rotation = match (lc, cr) {
                (Some(Odr::Greater), Some(Odr::Greater)) => left_turn_rate,
                (Some(Odr::Less), Some(Odr::Less)) => right_turn_rate,
                (Some(Odr::Less), Some(Odr::Greater)) => unit_rot,
                /*(Odr::Greater, Odr::Less) =>
                *[left_turn_rate, unit_rot, right_turn_rate]
                .choose(&mut rng)
                .unwrap(),*/
                _ => unit_rot,
            };

            // Integrate rotation
            let heading = rotation * f.heading;

            // Integrate position
            let position = f.position + heading * cfg.move_speed * dt;

            // Drop some slime (or create a new particle if out of bounds)
            if let Some(pos) = sample_array_vect(&self.back.medium, position) {
                self.back.medium[pos] += cfg.deposit_rate * dt;

                *b = SlimeParticle { position, heading };
            } else {
                *b = self.factory.slime(&mut rng);
            }
        }
    }
}

fn sample_array_isize<T: Copy>(arr: &Array2D<T>, x: isize, y: isize) -> Option<T> {
    let bounds = |x: isize, w: usize| {
        (x >= 0 && x < w as isize) //
            .then(|| x as usize)
    };

    let pos = (bounds(x, arr.width())?, bounds(y, arr.height())?);

    Some(arr[pos])
}

fn sample_array_vect<T>(arr: &Array2D<T>, v: Vector2<f32>) -> Option<(usize, usize)> {
    let bounds = |x: f32, w: usize| {
        (x.is_finite() && x >= 0. && x < w as f32) //
            .then(|| x as usize)
    };

    Some((bounds(v.x, arr.width())?, bounds(v.y, arr.height())?))
}

fn draw_sim(gb: &mut GraphicsBuilder, sim: &SlimeSim) {
    draw_grid_fuzzy(gb, &sim.frame().medium, |&v| [v; 3], 0.);
}

// Overengineered bullshit
struct SlimeFactory {
    x: Uniform<f32>,
    y: Uniform<f32>,
    angle: Uniform<f32>,
}

impl SlimeFactory {
    pub fn new(width: usize, height: usize) -> Self {
        let x = Uniform::new(0.0, width as f32);
        let y = Uniform::new(0.0, height as f32);
        let angle = Uniform::new(0., TAU);
        Self { x, y, angle }
    }

    pub fn slime(&self, mut rng: impl Rng) -> SlimeParticle {
        SlimeParticle {
            position: Vector2::new(self.x.sample(&mut rng), self.y.sample(&mut rng)),
            //position: Vector2::new(200., 200.), //Vector2::new(self.x.sample(&mut rng), self.y.sample(&mut rng)),
            heading: unit_circ(self.angle.sample(&mut rng)),
        }
    }
}


fn mix(a: f32, b: f32, t: f32) -> f32 {
    (1. - t) * a + t * b
}

