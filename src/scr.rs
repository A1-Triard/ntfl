#![deny(warnings)]

use either::Either;

#[derive(Debug, Copy, Clone)]
#[repr(i8)]
pub enum Color {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}

bitflags! {
pub struct Attr: u32 {
    const NORMAL = 0;
    const STANDOUT = 1 << 0;
    const UNDERLINE = 1 << 1;
    const REVERSE = 1 << 2;
    const BLINK = 1 << 3;
    const DIM = 1 << 4;
    const BOLD = 1 << 5;
    const ALTCHARSET = 1 << 6;
    const INVIS = 1 << 7;
    const PROTECT = 1 << 8;
    const HORIZONTAL = 1 << 9;
    const LEFT = 1 << 10;
    const LOW = 1 << 11;
    const RIGHT = 1 << 12;
    const TOP = 1 << 13;
    const VERTICAL = 1 << 14;
}
}

pub trait Scr {
    fn get_width(&self) -> Result<isize, ()>;
    fn get_height(&self) -> Result<isize, ()>;
    fn out(&mut self, y: isize, x: isize, ch: char, attr: Attr, fg: Color, bg: Option<Color>) -> Result<(), ()>;
    fn refresh(&mut self, cursor: Option<(isize, isize)>) -> Result<(), ()>;
    fn getch(&mut self) -> Result<Either<u32, char>, ()>;
}
