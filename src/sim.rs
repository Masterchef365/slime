use fruid::{DensitySim, FluidSim};
use idek_basics::Array2D;
use nalgebra::{Rotation2, Vector1, Vector2};
use rand::{distributions::Uniform, prelude::*};
use std::f32::consts::TAU;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};

#[derive(Clone, Default, Debug, StructOpt)]
pub struct SlimeConfig {
    /// Angle between adjacent sensors (radians)
    #[structopt(short = "s", long, default_value = "0.8")]
    sensor_spread: f32,

    /// Turn rate, radians/time
    #[structopt(short = "r", long, default_value = "1.8")]
    turn_speed: f32,

    /// Trail/slime decay rate
    #[structopt(short = "d", long, default_value = "0.05")]
    decay: f32,

    /// Deposit rate for slime
    #[structopt(short = "e", long, default_value = "1.0")]
    deposit_rate: f32,

    /// Slime movement speed
    #[structopt(short = "m", long, default_value = "1.0")]
    move_speed: f32,

    /// Sample distance
    #[structopt(short = "u", long, default_value = "3.0")]
    sample_dist: f32,

    /// Diffusion rate of the medium
    #[structopt(short = "i", long, default_value = "1e-10")]
    diffusion: f32,

    /// Viscosity of the medium
    #[structopt(short = "v", long, default_value = "0.")]
    viscosoty: f32,

    /// Random death rate
    #[structopt(short = "q", long, default_value = "0.01")]
    death_rate: f64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SlimeParticle {
    pub position: Vector2<f32>,
    pub heading: Vector2<f32>,
    pub origin: Vector2<f32>,
    pub age: u32,
}

#[derive(Clone)]
pub struct SlimeData {
    pub slime: Vec<SlimeParticle>,
}

pub struct SlimeSim {
    fluid: FluidSim,
    medium: DensitySim,
    /// The buffer to be presented to the user and read by the sim
    front: SlimeData,
    /// The buffer which is written to by the sim and swapped to front each frame
    back: SlimeData,
    /// Slime factory
    factory: SlimeFactory,
    time: f32,
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
        };

        let medium = DensitySim::new(width, height);

        let fluid = FluidSim::new(width, height);

        Self {
            fluid,
            medium,
            back: front.clone(),
            front,
            factory,
            time: 0.,
        }
    }

    pub fn frame(&self) -> (&SlimeData, &Array2D<f32>) {
        (&self.front, self.medium.density())
    }

    pub fn step(&mut self, cfg: &SlimeConfig, dt: f32, mut rng: impl Rng) {
        self.medium.density_mut().data_mut().fill(0.0);

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
                .map(|p| sample_array_vect(&self.medium.density(), p))
                .map(|p| p.map(|p| self.medium.density()[p]));

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

            let position = wraparound(self.medium.density(), position);

            // Happy birthday!
            let age = f.age + 1;

            let mut newparticle = rng.gen_bool(cfg.death_rate);

            // Drop some slime (or create a new particle if out of bounds)
            if let Some(pos) = sample_array_vect(&self.medium.density(), position) {
                self.medium.density_mut()[pos] += cfg.deposit_rate * dt;
                *b = SlimeParticle {
                    origin: f.origin,
                    position,
                    heading,
                    age,
                };
            } else {
                newparticle = true;
            }

            if newparticle {
                *b = self.factory.slime(&mut rng);
            }
        }

        // TODO: Slow hack!
        let d = self.medium.density().clone();
        self.medium.density_mut().data_mut().iter_mut().zip(d.data()).for_each(|(m, d)| *m -= *d * (1. - cfg.decay));

        // Fluid sim
        let (u, v) = self.fluid.uv_mut();
        let width = u.width() as f32;

        self.time += std::f32::consts::PI / width;
        let time: f32 = self.time as f32;

        let pos = (u.width() / 2, u.height() / 2);
        let m = 1. * width;// / 2.;
        u[pos] = -m * time.cos();
        v[pos] = -m * time.sin();

        let fluid_dt = 1e-2;

        self.fluid.step(fluid_dt, cfg.viscosoty);
        self.medium.step(self.fluid.uv(), fluid_dt, cfg.diffusion);

        std::mem::swap(&mut self.front, &mut self.back);
    }
}

fn sample_array_isize<T: Copy>(arr: &Array2D<T>, x: isize, y: isize) -> Option<T> {
    let bounds = |x: isize, w: usize| {
        Some((x % w as isize) as usize)
        //(x >= 0 && x < w as isize) //
            //.then(|| x as usize)
    };

    let pos = (bounds(x, arr.width())?, bounds(y, arr.height())?);

    Some(arr[pos])
}

pub fn wraparound<T>(arr: &Array2D<T>, v: Vector2<f32>) -> Vector2<f32> {
    Vector2::new(
        v.x.rem_euclid(arr.width() as f32), 
        v.y.rem_euclid(arr.height() as f32)
    )
}

/// If the given vector is within the boundaries of the array, return it's coordinates. Does not
/// modify the array!
pub fn sample_array_vect<T>(arr: &Array2D<T>, v: Vector2<f32>) -> Option<(usize, usize)> {
    let bounds = |x: f32, w: usize| {
        Some(x as usize % w)
    };

    Some((bounds(v.x, arr.width())?, bounds(v.y, arr.height())?))
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
        let origin = Vector2::new(self.x.sample(&mut rng), self.y.sample(&mut rng));
        SlimeParticle {
            position: origin,
            origin,
            //position: Vector2::new(200., 200.), //Vector2::new(self.x.sample(&mut rng), self.y.sample(&mut rng)),
            heading: unit_circ(self.angle.sample(&mut rng)),
            age: 0,
        }
    }
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    (1. - t) * a + t * b
}

