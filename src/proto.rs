use binrw::{BinWrite, io::NoSeek};
use smallvec::SmallVec;

use crate::animations::IsFrame;

#[derive(Clone, BinWrite)]
#[bw(big)]
pub struct BwFrame([u8; 39]);

impl BwFrame {
    pub fn new() -> Self {
        Self([0; 39])
    }

    pub fn merge(mut self, other: Self) -> Self {
        for idx in 0..39 {
            self.0[idx] |= other.0[idx];
        }
        self
    }
}

impl IsFrame for BwFrame {
    type Pixel = bool;

    fn get(&self, x: u8, y: u8) -> bool {
        let idx = (y as usize) * 9 + (x as usize);
        ((self.0[idx / 8] >> (idx % 8)) & 1) != 0
    }

    fn set(&mut self, x: u8, y: u8, value: bool) {
        let idx = (y as usize) * 9 + (x as usize);
        if value {
            self.0[idx / 8] |= 1 << ((idx % 8) as u8);
        } else {
            self.0[idx / 8] &= !(1 << ((idx % 8) as u8));
        }
    }
}

impl Default for BwFrame {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BinWrite)]
#[bw(big)]
#[repr(u8)]
pub enum Pattern {
    #[bw(magic = 0x0u8)]
    Percentage(u8),
    #[bw(magic = 0x1u8)]
    Gradient,
    #[bw(magic = 0x2u8)]
    DoubleGradient,
    #[bw(magic = 0x3u8)]
    LotusHorizontal,
    #[bw(magic = 0x4u8)]
    ZigZag,
    #[bw(magic = 0x5u8)]
    FullBrightness,
    #[bw(magic = 0x6u8)]
    Panic,
    #[bw(magic = 0x7u8)]
    LotusVertical,
}

#[derive(BinWrite)]
#[bw(big)]
#[repr(u8)]
#[bw(magic = 0x32ACu16)]
pub enum Command<'a> {
    #[bw(magic = 0x0u8)]
    SetBrightness(u8),
    #[bw(magic = 0x1u8)]
    Pattern(Pattern),
    #[bw(magic = 0x2u8)]
    Bootloader,
    #[bw(magic = 0x3u8)]
    Sleep(u8),
    #[bw(magic = 0x3u8)]
    GetSleep,
    #[bw(magic = 0x4u8)]
    Animate(u8),
    #[bw(magic = 0x4u8)]
    GetAnimate,
    #[bw(magic = 0x5u8)]
    Panic,
    #[bw(magic = 0x6u8)]
    DrawBw(&'a BwFrame),
    #[bw(magic = 0x7u8)]
    StageCol(u8, &'a [u8; 34]),
    #[bw(magic = 0x8u8)]
    FlushCols,
    #[bw(magic = 0x10u8)]
    StartGame(u8),
    #[bw(magic = 0x11u8)]
    GameCtrl(u8),
    #[bw(magic = 0x12u8)]
    GameStatus,
    #[bw(magic = 0x20u8)]
    Version,
}

impl Command<'_> {
    pub fn response_size(&self) -> usize {
        match self {
            Self::GetSleep => 1,
            Self::GetAnimate => 1,
            Self::Version => 3,
            _ => 0,
        }
    }

    pub fn to_bytes(&self) -> SmallVec<[u8; 64]> {
        let mut buf = NoSeek::new(SmallVec::new());
        self.write(&mut buf).unwrap();
        buf.into_inner()
    }
}
