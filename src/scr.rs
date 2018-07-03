#![deny(warnings)]

use either::Either;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Key {
    pub value: u32
}

impl Key {
    pub const MIN: Key = Key { value: 0o401 }; // Minimum curses key
    pub const BREAK: Key = Key { value: 0o401 }; // Break key (unreliable)
    pub const SRESET: Key = Key { value: 0o530 }; // Soft (partial) reset (unreliable)
    pub const RESET: Key = Key { value: 0o531 }; // Reset or hard reset (unreliable)
    pub const DOWN: Key = Key { value: 0o402 }; // down-arrow key
    pub const UP: Key = Key { value: 0o403 }; // up-arrow key
    pub const LEFT: Key = Key { value: 0o404 }; // left-arrow key
    pub const RIGHT: Key = Key { value: 0o405 }; // right-arrow key
    pub const HOME: Key = Key { value: 0o406 }; // home key
    pub const BACKSPACE: Key = Key { value: 0o407 }; // backspace key
    pub const F0: Key = Key { value: 0o410 }; // Function keys.  Space for 64
    pub fn f(n: u8) -> Key { Key { value: Key::F0.value + (n as u32) } } // Value of function key n
    pub const DL: Key = Key { value: 0o510 }; // delete-line key
    pub const IL: Key = Key { value: 0o511 }; // insert-line key
    pub const DC: Key = Key { value: 0o512 }; // delete-character key
    pub const IC: Key = Key { value: 0o513 }; // insert-character key
    pub const EIC: Key = Key { value: 0o514 }; // sent by rmir or smir in insert mode
    pub const CLEAR: Key = Key { value: 0o515 }; // clear-screen or erase key
    pub const EOS: Key = Key { value: 0o516 }; // clear-to-end-of-screen key
    pub const EOL: Key = Key { value: 0o517 }; // clear-to-end-of-line key
    pub const SF: Key = Key { value: 0o520 }; // scroll-forward key
    pub const SR: Key = Key { value: 0o521 }; // scroll-backward key
    pub const NPAGE: Key = Key { value: 0o522 }; // next-page key
    pub const PPAGE: Key = Key { value: 0o523 }; // previous-page key
    pub const STAB: Key = Key { value: 0o524 }; // set-tab key
    pub const CTAB: Key = Key { value: 0o525 }; // clear-tab key
    pub const CATAB: Key = Key { value: 0o526 }; // clear-all-tabs key
    pub const ENTER: Key = Key { value: 0o527 }; // enter/send key
    pub const PRINT: Key = Key { value: 0o532 }; // print key
    pub const LL: Key = Key { value: 0o533 }; // lower-left key (home down)
    pub const A1: Key = Key { value: 0o534 }; // upper left of keypad
    pub const A3: Key = Key { value: 0o535 }; // upper right of keypad
    pub const B2: Key = Key { value: 0o536 }; // center of keypad
    pub const C1: Key = Key { value: 0o537 }; // lower left of keypad
    pub const C3: Key = Key { value: 0o540 }; // lower right of keypad
    pub const BTAB: Key = Key { value: 0o541 }; // back-tab key
    pub const BEG: Key = Key { value: 0o542 }; // begin key
    pub const CANCEL: Key = Key { value: 0o543 }; // cancel key
    pub const CLOSE: Key = Key { value: 0o544 }; // close key
    pub const COMMAND: Key = Key { value: 0o545 }; // command key
    pub const COPY: Key = Key { value: 0o546 }; // copy key
    pub const CREATE: Key = Key { value: 0o547 }; // create key
    pub const END: Key = Key { value: 0o550 }; // end key
    pub const EXIT: Key = Key { value: 0o551 }; // exit key
    pub const FIND: Key = Key { value: 0o552 }; // find key
    pub const HELP: Key = Key { value: 0o553 }; // help key
    pub const MARK: Key = Key { value: 0o554 }; // mark key
    pub const MESSAGE: Key = Key { value: 0o555 }; // message key
    pub const MOVE: Key = Key { value: 0o556 }; // move key
    pub const NEXT: Key = Key { value: 0o557 }; // next key
    pub const OPEN: Key = Key { value: 0o560 }; // open key
    pub const OPTIONS: Key = Key { value: 0o561 }; // options key
    pub const PREVIOUS: Key = Key { value: 0o562 }; // previous key
    pub const REDO: Key = Key { value: 0o563 }; // redo key
    pub const REFERENCE: Key = Key { value: 0o564 }; // reference key
    pub const REFRESH: Key = Key { value: 0o565 }; // refresh key
    pub const REPLACE: Key = Key { value: 0o566 }; // replace key
    pub const RESTART: Key = Key { value: 0o567 }; // restart key
    pub const RESUME : Key = Key { value: 0o570 }; // resume key
    pub const SAVE: Key = Key { value: 0o571 }; // save key
    pub const SBEG: Key = Key { value: 0o572 }; // shifted begin key
    pub const SCANCEL: Key = Key { value: 0o573 }; // shifted cancel key
    pub const SCOMMAND: Key = Key { value: 0o574 }; // shifted command key
    pub const SCOPY: Key = Key { value: 0o575 }; // shifted copy key
    pub const SCREATE: Key = Key { value: 0o576 }; // shifted create key
    pub const SDC: Key = Key { value: 0o577 }; // shifted delete-character key
    pub const SDL: Key = Key { value: 0o600 }; // shifted delete-line key
    pub const SELECT: Key = Key { value: 0o601 }; // select key
    pub const SEND: Key = Key { value: 0o602 }; // shifted end key
    pub const SEOL: Key = Key { value: 0o603 }; // shifted clear-to-end-of-line key
    pub const SEXIT: Key = Key { value: 0o604 }; // shifted exit key
    pub const SFIND: Key = Key { value: 0o605 }; // shifted find key
    pub const SHELP: Key = Key { value: 0o606 }; // shifted help key
    pub const SHOME: Key = Key { value: 0o607 }; // shifted home key
    pub const SIC: Key = Key { value: 0o610 }; // shifted insert-character key
    pub const SLEFT: Key = Key { value: 0o611 }; // shifted left-arrow key
    pub const SMESSAGE: Key = Key { value: 0o612 }; // shifted message key
    pub const SMOVE: Key = Key { value: 0o613 }; // shifted move key
    pub const SNEXT: Key = Key { value: 0o614 }; // shifted next key
    pub const SOPTIONS: Key = Key { value: 0o615 }; // shifted options key
    pub const SPREVIOUS: Key = Key { value: 0o616 }; // shifted previous key
    pub const SPRINT: Key = Key { value: 0o617 }; // shifted print key
    pub const SREDO: Key = Key { value: 0o620 }; // shifted redo key
    pub const SREPLACE: Key = Key { value: 0o621 }; // shifted replace key
    pub const SRIGHT: Key = Key { value: 0o622 }; // shifted right-arrow key
    pub const SRSUME: Key = Key { value: 0o623 }; // shifted resume key
    pub const SSAVE: Key = Key { value: 0o624 }; // shifted save key
    pub const SSUSPEND: Key = Key { value: 0o625 }; // shifted suspend key
    pub const SUNDO: Key = Key { value: 0o626 }; // shifted undo key
    pub const SUSPEND: Key = Key { value: 0o627 }; // suspend key
    pub const UNDO: Key = Key { value: 0o630 }; // undo key
    pub const MOUSE: Key = Key { value: 0o631 }; // Mouse event has occurred
    pub const RESIZE: Key = Key { value: 0o632 }; // Terminal resize event
    pub const EVENT: Key = Key { value: 0o633 }; // We were interrupted by an event
    pub const MAX: Key = Key { value: 0o777 }; // Maximum key value is 0o633
}



#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn getch(&mut self) -> Result<Either<Key, char>, ()>;
}

#[cfg(test)]
pub mod tests {
    use std::mem::replace;
    use either::Either;
    use scr::{ Attr, Texel, Color, Scr, Key };

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
            replace(&mut self.content[(y * self.width + x) as usize], c.clone());
            Ok(())
        }
        fn refresh(&mut self, cursor: Option<(isize, isize)>) -> Result<(), ()> {
            self.invalid = false;
            self.cursor = cursor;
            Ok(())
        }
        fn getch(&mut self) -> Result<Either<Key, char>, ()> {
            Err(())
        }
    }
}
