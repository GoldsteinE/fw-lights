use std::{
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Instant,
};

use crate::{
    MatrixPort,
    animations::Animation,
    proto::{BwFrame, Command},
};

pub enum DisplayCommand {
    SetBrightness(u8),
    AddAnimation(Animation),
}

pub struct Matrix {
    port: MatrixPort,
    brightness: u8,
    animations: Vec<Animation>,
}

impl Matrix {
    pub fn new(port: MatrixPort) -> eyre::Result<Self> {
        let animations = Vec::with_capacity(16);
        Ok(Self {
            port,
            animations,
            brightness: 255,
        })
    }

    fn set_brightness(&mut self, brightness: u8) -> eyre::Result<()> {
        self.brightness = brightness;
        self.port
            .send_command(Command::SetBrightness(self.brightness))?;
        Ok(())
    }

    fn process_command(&mut self, command: DisplayCommand) -> eyre::Result<()> {
        match command {
            DisplayCommand::SetBrightness(brightness) => self.set_brightness(brightness),
            DisplayCommand::AddAnimation(animation) => {
                self.animations.push(animation);
                Ok(())
            }
        }
    }

    pub fn run(&mut self, rx: mpsc::Receiver<DisplayCommand>) -> eyre::Result<()> {
        // normalize current brightness
        self.set_brightness(self.brightness)?;
        let mut frames = Vec::with_capacity(16);
        loop {
            if let Ok(command) = rx.try_recv() {
                self.process_command(command)?
            }

            self.animations.retain_mut(|animation| {
                if let Some(frame) = animation.next() {
                    frames.push(frame);
                    true
                } else {
                    false
                }
            });

            let frame = frames
                .drain(..)
                .reduce(|lower, upper| lower.merge(upper, self.brightness));

            let Some(frame) = frame else {
                // draw an empty frame to reset display
                self.port
                    .send_command(Command::DrawBw(&BwFrame::default()))?;
                let Ok(command) = rx.recv() else {
                    return Ok(());
                };
                self.process_command(command)?;
                continue;
            };

            let before = Instant::now();
            self.port.draw_frame(&frame)?;
            thread::sleep(frame.min_duration.saturating_sub(before.elapsed()));
        }
    }

    pub fn spawn(mut self) -> (mpsc::Sender<DisplayCommand>, JoinHandle<eyre::Result<()>>) {
        let (tx, rx) = mpsc::channel();
        (tx, thread::spawn(move || self.run(rx)))
    }
}
