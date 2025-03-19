use std::{collections::HashMap, time::Duration};

use crate::{
    animations::{Animation, Frame, FrameData, GrayFrame},
    config,
};

pub enum Diagonals {
    Yes,
    No,
    Costly(u16, u16),
}

pub struct Spread<F> {
    // inefficient, but who cares
    current_set: HashMap<(u8, u8), u8>,
    buffer: HashMap<(u8, u8), u8>,
    duration: Duration,
    step: F,
}

pub fn from_config_at(config: config::SpreadAnimation, offset: i8) -> Animation {
    let mut result = Spread::new(config.frame_duration, move |dy, dx| match (dy, dx) {
        (0, 0) => config.stay_cost,
        (_, 0) => config.horiz_cost,
        (0, _) => config.vert_cost,
        _ => config.diag_cost,
    });

    for [x, y, brightness] in config.seeds {
        let y = if offset < 0 {
            y.checked_sub((-offset) as u8)
        } else {
            y.checked_add(offset as u8).filter(|&y| y < 34)
        };
        // out of bounds, ignore
        let Some(y) = y else {
            continue;
        };
        result.set(y, x, brightness);
    }

    Box::new(result)
}

impl<F> Spread<F> {
    pub fn new(duration: Duration, step: F) -> Self {
        Self {
            current_set: HashMap::with_capacity(34 * 9),
            buffer: HashMap::with_capacity(34 * 9),
            duration,
            step,
        }
    }

    pub fn set(&mut self, y: u8, x: u8, brightness: u8) {
        self.current_set.insert((y, x), brightness);
    }

    pub fn is_empty(&self) -> bool {
        self.current_set.is_empty()
    }

    pub fn to_frame(&self) -> GrayFrame {
        let mut frame = GrayFrame::default();
        for (&(y, x), &brightness) in &self.current_set {
            frame.0[x as usize][y as usize] = brightness;
        }
        frame
    }
}

impl<F> Iterator for Spread<F>
where
    F: FnMut(i8, i8) -> u8,
{
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_empty() {
            return None;
        }

        let frame = Frame {
            data: FrameData::Gray(self.to_frame()),
            min_duration: self.duration,
            fullscreen: false,
        };
        self.buffer.clear();
        // terrible algo, but again, who cares
        for (&(y, x), &brightness) in &self.current_set {
            let sx = x.saturating_sub(1);
            let sy = y.saturating_sub(1);
            let ex = x + 1;
            let ey = y + 1;
            for nx in sx..=ex {
                for ny in sy..=ey {
                    if !(0..=33).contains(&ny) || !(0..=8).contains(&nx) {
                        continue;
                    }

                    let step = (self.step)(ny as i8 - y as i8, nx as i8 - x as i8);

                    if brightness <= step {
                        continue;
                    }

                    let cell = self.buffer.entry((ny, nx)).or_default();
                    *cell = (*cell).max(brightness - step);
                }
            }
        }
        std::mem::swap(&mut self.current_set, &mut self.buffer);
        Some(frame)
    }
}
