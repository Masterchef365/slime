use idek_basics::Array3D;
use nalgebra::{Rotation2, Rotation3, Unit, UnitQuaternion, Vector1, Vector2, Vector3};
use rand::{distributions::Uniform, prelude::*};
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};
use structopt::StructOpt;

type Pos = (isize, isize, isize);

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
    #[structopt(short = "m", long, default_value = "0.1")]
    move_speed: f32,

    /// Sample distance
    #[structopt(short = "u", long, default_value = "3.0")]
    sample_dist: f32,

    /// Dive rate
    #[structopt(short = "p", long, default_value = "1.0")]
    dive_rate: f32,

    /// Diffusion rate of the medium
    #[structopt(short = "i", long, default_value = "0.1")]
    diffusion: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct SlimeParticle {
    pub position: Vector3<f32>,
    pub heading: UnitQuaternion<f32>,
    pub origin: Vector3<f32>,
    pub age: u32,
}

#[derive(Clone)]
pub struct SlimeData {
    pub medium: Array3D<f32>,
    pub slime: Vec<SlimeParticle>,
}

pub struct SlimeSim {
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
    pub fn new(
        width: usize,
        height: usize,
        length: usize,
        n_particles: usize,
        mut rng: impl Rng,
    ) -> Self {
        let factory = SlimeFactory::new(width, height, length);

        let slime = (0..n_particles).map(|_| factory.slime(&mut rng)).collect();

        let front = SlimeData {
            slime,
            medium: Array3D::new(width, height, length),
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

    pub fn step(&mut self, cfg: &SlimeConfig, dt: f32, mut rng: impl Rng) {
        self.step_medium(cfg, dt, &mut rng);
        self.step_particles(cfg, dt, &mut rng);
        std::mem::swap(&mut self.front, &mut self.back);
    }

    /// Diffusion and decay
    fn step_medium(&mut self, cfg: &SlimeConfig, dt: f32, mut rng: impl Rng) {
        for z in 0..self.front.medium.length() {
            for y in 0..self.front.medium.height() {
                for x in 0..self.front.medium.width() {
                    // TODO: break this out into a function. Don't nest so deep!
                    let mut sum = 0.;
                    let mut n_parts = 0;

                    let off = [
                        (0, 0, 0),

                        (1, 0, 0),
                        (-1, 0, 0),

                        (0, 1, 0),
                        (0, -1, 0),

                        (0, 0, 1),
                        (0, 0, -1),
                    ];

                    for (i, j, k) in off {
                        let sample_pos = (j + x as isize, i + y as isize, k + z as isize);
                        if let Some(v) = sample_array_isize(&self.front.medium, sample_pos)
                        {
                            sum += v;
                            n_parts += 1;
                        }
                    }

                    let avg = sum / n_parts as f32;

                    let pos = (x, y, z);
                    let center = self.front.medium[pos];

                    let diffuse = mix(center, avg, cfg.diffusion);

                    let decayed = (1. - cfg.decay) * diffuse;

                    self.back.medium[pos] = decayed;
                }
            }
        }
    }

    fn step_particles(&mut self, cfg: &SlimeConfig, dt: f32, mut rng: impl Rng) {
        let particle_tip = Vector3::z();

        let particle_yaw_axis = Unit::new_unchecked(Vector3::x());
        let particle_pitch_axis = Unit::new_unchecked(Vector3::y());

        let yaw_sensor = UnitQuaternion::from_axis_angle(&particle_yaw_axis, cfg.sensor_spread);
        let pitch_sensor = UnitQuaternion::from_axis_angle(&particle_pitch_axis, cfg.sensor_spread);

        let yaw_turn = UnitQuaternion::from_axis_angle(&particle_yaw_axis, cfg.turn_speed * dt);
        let pitch_turn = UnitQuaternion::from_axis_angle(&particle_pitch_axis, cfg.turn_speed * dt);

        let unit_rot = UnitQuaternion::identity();

        // Step particle motion
        for (b, f) in self.back.slime.iter_mut().zip(&self.front.slime) {
            // Sample a particle's sensor
            let sample = |part: &SlimeParticle, rot: UnitQuaternion<f32>| {
                let sensor_offset = rot * part.heading * particle_tip * cfg.sample_dist;
                sample_array_vect(&self.back.medium, part.position + sensor_offset)
                    .map(|p| self.front.medium[p])
            };

            // Sample using a rotation
            let rotsample = |sensor: UnitQuaternion<f32>, turn: UnitQuaternion<f32>| {
                // Sample the grid
                let [left, center, right] =
                    [sensor, unit_rot, sensor.inverse()].map(|r| sample(f, r));

                // Decide which way to go
                let lc = left.partial_cmp(&center);
                let cr = center.partial_cmp(&right);

                use std::cmp::Ordering as Odr;

                match (lc, cr) {
                    (Some(Odr::Greater), Some(Odr::Greater)) => turn,
                    (Some(Odr::Less), Some(Odr::Less)) => turn.inverse(),
                    _ => unit_rot,
                }
            };

            // Total descision
            let rotation = rotsample(yaw_sensor, yaw_turn) * rotsample(pitch_sensor, pitch_turn);

            // Integrate rotation
            let heading = rotation * f.heading;

            // Integrate position
            let position = f.position + (heading * particle_tip * cfg.move_speed * dt);

            // Happy birthday!
            let age = f.age + 1;

            // Drop some slime (or create a new particle if out of bounds)
            if let Some(pos) = sample_array_vect(&self.back.medium, position) {
                self.back.medium[pos] += cfg.deposit_rate * dt;
                *b = SlimeParticle {
                    origin: f.origin,
                    position,
                    heading,
                    age,
                };
            } else {
                *b = self.factory.slime(&mut rng);
            }
        }
    }
}

fn sample_array_isize<T: Copy>(arr: &Array3D<T>, (x, y, z): Pos) -> Option<T> {
    let bounds = |x: isize, w: usize| {
        (x >= 0 && x < w as isize) //
            .then(|| x as usize)
    };

    let pos = (
        bounds(x, arr.width())?,
        bounds(y, arr.height())?,
        bounds(z, arr.length())?,
    );

    Some(arr[pos])
}

fn sample_array_vect<T>(arr: &Array3D<T>, v: Vector3<f32>) -> Option<(usize, usize, usize)> {
    let bounds = |x: f32, w: usize| {
        (x.is_finite() && x >= 0. && x < w as f32) //
            .then(|| x as usize)
    };

    Some((
        bounds(v.x, arr.width())?,
        bounds(v.y, arr.height())?,
        bounds(v.z, arr.length())?,
    ))
}

// Overengineered bullshit
struct SlimeFactory {
    x: Uniform<f32>,
    y: Uniform<f32>,
    z: Uniform<f32>,
    theta: Uniform<f32>,
    rho: Uniform<f32>,
}

impl SlimeFactory {
    pub fn new(width: usize, height: usize, length: usize) -> Self {
        let x = Uniform::new(0.0, width as f32);
        let y = Uniform::new(0.0, height as f32);
        let z = Uniform::new(0.0, length as f32);

        let theta = Uniform::new(0., PI);
        let rho = Uniform::new(-PI, PI);

        Self {
            x,
            y,
            z,
            theta,
            rho,
        }
    }

    pub fn slime(&self, mut rng: impl Rng) -> SlimeParticle {
        let origin = Vector3::new(
            self.x.sample(&mut rng),
            self.y.sample(&mut rng),
            self.z.sample(&mut rng),
        );

        // Depends on PARTICLE_TIP!
        let heading = UnitQuaternion::from_euler_angles(
            self.theta.sample(&mut rng),
            self.rho.sample(&mut rng),
            0.,
        );

        SlimeParticle {
            position: origin,
            origin,
            heading,
            //position: Vector2::new(200., 200.), //Vector2::new(self.x.sample(&mut rng), self.y.sample(&mut rng)),
            age: 0,
        }
    }
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    (1. - t) * a + t * b
}
