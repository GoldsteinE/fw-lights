use std::time::Duration;

use crate::proto::BwFrame;

pub mod builder;

// specific animations
pub mod file;
pub mod spread;

pub type Animation = Box<dyn Iterator<Item = Frame> + Send + Sync>;

pub trait IsFrame: Default {
    type Pixel: Copy;

    fn get(&self, x: u8, y: u8) -> Self::Pixel;
    fn set(&mut self, x: u8, y: u8, pixel: Self::Pixel);

    fn offset(self, offset: i8) -> Self {
        let mut result = Self::default();
        for y in 0..34 {
            let Some(oy) = (y as i8).checked_sub(offset) else {
                continue;
            };
            let Ok(oy) = u8::try_from(oy) else {
                continue;
            };
            if oy > 33 {
                continue;
            }
            for x in 0..9 {
                result.set(x, y, self.get(x, oy));
            }
        }
        result
    }
}

#[derive(Clone)]
pub struct GrayFrame(pub [[u8; 34]; 9]);

impl GrayFrame {
    pub fn from_bw(bw: BwFrame, brightness: u8) -> Self {
        let mut result = Self::default();
        for x in 0..9 {
            for y in 0..34 {
                if bw.get(x, y) {
                    result.0[x as usize][y as usize] = brightness;
                }
            }
        }
        result
    }

    pub fn merge(mut self, other: Self) -> Self {
        for x in 0..9 {
            for y in 0..34 {
                self.0[x][y] = self.0[x][y].max(other.0[x][y]);
            }
        }
        self
    }
}

impl IsFrame for GrayFrame {
    type Pixel = u8;

    fn get(&self, x: u8, y: u8) -> Self::Pixel {
        self.0[x as usize][y as usize]
    }

    fn set(&mut self, x: u8, y: u8, pixel: Self::Pixel) {
        self.0[x as usize][y as usize] = pixel;
    }
}

impl Default for GrayFrame {
    fn default() -> Self {
        Self([[0; 34]; 9])
    }
}

#[allow(clippy::large_enum_variant)] // maybe actually box? dunno
#[derive(Clone)]
pub enum FrameData {
    Gray(GrayFrame),
    Bw(BwFrame),
}

#[derive(Clone)]
pub struct Frame {
    pub data: FrameData,
    pub min_duration: Duration,
    pub fullscreen: bool,
}

impl Frame {
    pub fn merge(self, upper: Frame, bw_brightness: u8) -> Frame {
        if upper.fullscreen {
            return upper;
        }

        // fullscreen frames always win
        if self.fullscreen {
            return self;
        }

        let data = match (self.data, upper.data) {
            (FrameData::Bw(lower), FrameData::Bw(upper)) => FrameData::Bw(lower.merge(upper)),
            (FrameData::Gray(lower), FrameData::Gray(upper)) => FrameData::Gray(lower.merge(upper)),
            (FrameData::Bw(lower), FrameData::Gray(upper)) => {
                FrameData::Gray(GrayFrame::from_bw(lower, bw_brightness).merge(upper))
            }
            (FrameData::Gray(lower), FrameData::Bw(upper)) => {
                FrameData::Gray(lower.merge(GrayFrame::from_bw(upper, bw_brightness)))
            }
        };

        Self {
            data,
            min_duration: self.min_duration.max(upper.min_duration),
            // merged frames are definitionally never fullscreen
            fullscreen: false,
        }
    }

    pub fn offset(self, offset: i8) -> Self {
        let Self {
            data,
            min_duration,
            fullscreen,
        } = self;
        let data = match data {
            FrameData::Gray(frame) => FrameData::Gray(frame.offset(offset)),
            FrameData::Bw(frame) => FrameData::Bw(frame.offset(offset)),
        };
        Self {
            data,
            min_duration,
            fullscreen,
        }
    }
}
