use std::{path::Path, io::{BufReader, BufWriter}, fs::File};
use idek_basics::idek::prelude::Result;
use crate::sim::{SlimeParticle, SlimeSim};
use serde::{Serialize, Deserialize};

pub fn record_frame(record: &mut RecordFile, sim: &SlimeSim) {
    let slime = sim.frame().slime.clone();
    record.frames.push(RecordFrame { slime });
}

#[derive(Default, Serialize, Deserialize)]
pub struct RecordFile {
    pub width: usize,
    pub height: usize,
    pub frames: Vec<RecordFrame>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct RecordFrame {
    pub slime: Vec<SlimeParticle>,
}

impl RecordFile {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            frames: vec![],
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let reader = BufReader::new(File::open(path)?);
        Ok(bincode::deserialize_from(reader)?)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let writer = BufWriter::new(File::create(path)?);
        Ok(bincode::serialize_into(writer, self)?)
    }
}
