#![deny(warnings)]

use either::Either;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Texel {
    pub ch: char,
    pub attr: Attr,
    pub fg: Color,
    pub bg: Option<Color>,
}

pub trait Scr {
    fn get_height(&self) -> Result<isize, ()>;
    fn get_width(&self) -> Result<isize, ()>;
    fn out(&mut self, y: isize, x: isize, c: &Texel) -> Result<(), ()>;
    fn refresh(&mut self, cursor: Option<(isize, isize)>) -> Result<(), ()>;
    fn getch(&mut self) -> Result<Either<u32, char>, ()>;
}

#[cfg(test)]
pub mod tests {
    use std::mem::replace;
    use either::Either;
    use Attr;
    use Texel;
    use Color;
    use Scr;

    pub struct TestScr {
        pub height: isize,
        pub width: isize,
        pub invalid: bool,
        pub content: Vec<Texel>,
        pub cursor: Option<(isize, isize)>,
    }
    impl TestScr {
        pub fn new(height: isize, width: isize) -> TestScr {
            TestScr {
                height: height,
                width: width,
                invalid: false,
                content: vec![Texel { ch: 'T', attr: Attr::NORMAL, fg: Color::Cyan, bg: Some(Color::Red) }; (height * width) as usize],
                cursor: None
            }
        }
        pub fn content(&self, y: isize, x: isize) -> &Texel {
            &self.content[(y * self.width + x) as usize]
        }
    }
    impl Scr for TestScr {
        fn get_height(&self) -> Result<isize, ()> { Ok(self.height) }
        fn get_width(&self) -> Result<isize, ()> { Ok(self.width) }
        fn out(&mut self, y: isize, x: isize, c: &Texel) -> Result<(), ()> {
            self.invalid = true;
            replace(&mut self.content[(y * self.width + x) as usize], *c);
            Ok(())
        }
        fn refresh(&mut self, cursor: Option<(isize, isize)>) -> Result<(), ()> {
            self.invalid = false;
            self.cursor = cursor;
            Ok(())
        }
        fn getch(&mut self) -> Result<Either<u32, char>, ()> {
            Err(())
        }
    }
}
