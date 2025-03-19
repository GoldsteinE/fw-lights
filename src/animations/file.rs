use std::{iter, str::FromStr, time::Duration};

use eyre::{bail, ensure};
use itertools::Itertools as _;
use serde::Deserialize;

use crate::{
    animations::{Animation, Frame, FrameData, GrayFrame, IsFrame as _},
    proto::BwFrame,
};

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct FileOptions {
    pub default_offset: i8,
    #[serde(with = "humantime_serde")]
    pub min_duration: Duration,
    pub fullscreen: bool,
}

pub struct FileAnimation {
    pub frames: Vec<Frame>,
    pub default_offset: i8,
}

impl FileAnimation {
    pub fn at(&self, offset: Option<i8>) -> Animation {
        let offset = offset.unwrap_or(self.default_offset);
        Box::new(
            self.frames
                .clone()
                .into_iter()
                .map(move |frame| frame.offset(offset)),
        )
    }
}

impl FromStr for FileAnimation {
    type Err = eyre::Report;

    fn from_str(s: &str) -> eyre::Result<Self> {
        let Some(header_delim) = s.find("\n---\n") else {
            bail!("animation file doesn't contain `---` line");
        };

        let options: FileOptions = toml::from_str(&s[..header_delim])?;
        let default_frame_options = FrameOptions {
            repeat: None,
            fullscreen: Some(options.fullscreen),
            min_duration: Some(options.min_duration),
        };
        let data = &s[header_delim + 5..];
        let first_line_idx = s[..header_delim].bytes().filter(|&b| b == b'\n').count() + 3;
        let mut lines = data
            .lines()
            .enumerate()
            .map(|(n, line)| (n + first_line_idx, line));
        let mut frames = Vec::new();
        while parse_frame(&mut lines, &default_frame_options, &mut frames)? {}

        Ok(Self {
            frames,
            default_offset: options.default_offset,
        })
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct FrameOptions {
    pub repeat: Option<usize>,
    pub fullscreen: Option<bool>,
    #[serde(with = "humantime_serde::option")]
    pub min_duration: Option<Duration>,
}

impl FrameOptions {
    pub fn merge_with(&mut self, other: &FrameOptions) {
        if other.repeat.is_some() {
            self.repeat = other.repeat;
        }
        if other.fullscreen.is_some() {
            self.fullscreen = other.fullscreen;
        }
        if other.min_duration.is_some() {
            self.min_duration = other.min_duration;
        }
    }

    pub fn make_bw(self, frame: BwFrame) -> Frame {
        Frame {
            data: FrameData::Bw(frame),
            min_duration: self.min_duration.unwrap_or_default(),
            fullscreen: self.fullscreen.unwrap_or(false),
        }
    }

    pub fn make_gray(self, frame: GrayFrame) -> Frame {
        Frame {
            data: FrameData::Gray(frame),
            min_duration: self.min_duration.unwrap_or_default(),
            fullscreen: self.fullscreen.unwrap_or(false),
        }
    }
}

fn parse_frame<'a>(
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
    default_options: &FrameOptions,
    into: &mut Vec<Frame>,
) -> eyre::Result<bool> {
    let mut options = default_options.clone();
    loop {
        let Some((n, line)) = lines.next() else {
            return Ok(false);
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // options line
        if line.as_bytes()[0] == b'{' {
            let frame_options = FrameOptions::deserialize(toml::de::ValueDeserializer::new(line))?;
            options.merge_with(&frame_options);
            continue;
        }

        let mut lines = iter::once((n, line)).chain(lines);

        let repeat = options.repeat;
        let frame = if line.as_bytes()[0] == b'.' || line.as_bytes()[0] == b'#' {
            parse_bw(&mut lines, options)?
        } else {
            parse_gray(&mut lines, options)?
        };

        for _ in 0..repeat.unwrap_or(1) {
            into.push(frame.clone());
        }
        return Ok(true);
    }
}

fn parse_bw<'a>(
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
    options: FrameOptions,
) -> Result<Frame, eyre::Error> {
    let mut frame = BwFrame::default();
    for (y, (n, line)) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            break;
        }

        ensure!(y < 34, "line {n}: too many lines in frame");
        let Some(pixels) = line
            .bytes()
            .filter(|c| !c.is_ascii_whitespace())
            .collect_array::<9>()
        else {
            bail!("line {n}: wrong frame line length");
        };
        for (x, pixel) in pixels.into_iter().enumerate() {
            ensure!(
                pixel == b'.' || pixel == b'#',
                "line {n}: wrong pixel '{}': should be '.' or '#'",
                pixel as char,
            );
            // y cast is safe because of the ensure above
            // x cast is safe because it's an array index
            frame.set(x as u8, y as u8, pixel == b'#');
        }
    }
    Ok(options.make_bw(frame))
}

fn parse_gray<'a>(
    lines: &mut impl Iterator<Item = (usize, &'a str)>,
    options: FrameOptions,
) -> Result<Frame, eyre::Error> {
    let mut frame = GrayFrame::default();

    for (y, (n, line)) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            break;
        }

        ensure!(y < 34, "line {n}: too many lines in frame");
        let Some(pixels) = line.split_ascii_whitespace().collect_array::<9>() else {
            bail!("line {n}: wrong frame line length");
        };
        for (x, pixel) in pixels.into_iter().enumerate() {
            let Ok(pixel) = u8::from_str_radix(pixel, 16) else {
                bail!("line {n}: wrong pixel {pixel:?}");
            };
            frame.0[x][y] = pixel;
        }
    }

    Ok(options.make_gray(frame))
}
