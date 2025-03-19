#![allow(dead_code)]

use std::time::Duration;

use serialport::SerialPort;
use smallvec::SmallVec;

use crate::{
    animations::{Frame, FrameData, GrayFrame},
    proto::Command,
};

pub mod animations;
pub mod config;
pub mod display_thread;
pub mod proto;
pub mod daemon;

pub struct MatrixPort {
    port: Box<dyn SerialPort>,
}

impl MatrixPort {
    pub fn open(path: &str) -> eyre::Result<Self> {
        let port = serialport::new(path, 115200)
            .timeout(Duration::from_millis(1))
            .open()?;
        Ok(Self { port })
    }

    pub fn send_command(&mut self, command: Command<'_>) -> eyre::Result<SmallVec<[u8; 8]>> {
        self.port.write_all(&command.to_bytes())?;
        self.port.flush()?;
        let mut response = SmallVec::new();
        let response_size = command.response_size();
        if response_size != 0 {
            response.extend(std::iter::repeat_n(0, response_size));
            self.port.read_exact(&mut response)?;
        }
        Ok(response)
    }

    pub fn draw_gray_frame(&mut self, frame: &GrayFrame) -> eyre::Result<()> {
        for (x, column) in frame.0.iter().enumerate() {
            // TODO: do we need to do this check? it makes timing less consistent
            if column.iter().any(|&brightness| brightness != 0) {
                // cast is safe, as there're only 9 columns
                self.send_command(Command::StageCol(x as u8, column))?;
            }
        }
        self.send_command(Command::FlushCols)?;

        Ok(())
    }

    pub fn draw_frame(&mut self, frame: &Frame) -> eyre::Result<()> {
        match &frame.data {
            FrameData::Gray(gray_frame) => self.draw_gray_frame(gray_frame),
            FrameData::Bw(bw_frame) => self.send_command(Command::DrawBw(bw_frame)).map(drop),
        }
    }
}
