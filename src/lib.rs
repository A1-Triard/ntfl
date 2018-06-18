#![deny(warnings)]
extern crate libc;
extern crate either;
#[macro_use]
extern crate bitflags;

use std::char::from_u32;
use std::os::raw::{ c_int, c_void, c_short, c_char, c_uint };
use std::ptr::null;
use either::{ Either, Left, Right };
use libc::{ setlocale, LC_ALL };

include!(concat!(env!("OUT_DIR"), "/c_bool.rs"));
include!(concat!(env!("OUT_DIR"), "/ERR.rs"));
include!(concat!(env!("OUT_DIR"), "/attr_t.rs"));
include!(concat!(env!("OUT_DIR"), "/KEY_CODE_YES.rs"));

type WINDOW = c_void;

extern "C" {
    fn initscr() -> *mut WINDOW;
    fn endwin() -> c_int;
    fn noecho() -> c_int;
    fn wrefresh(w: *mut WINDOW) -> c_int;
    fn wmove(w: *mut WINDOW, y: c_int, x: c_int) -> c_int;
    fn waddnstr(w: *mut WINDOW, s: *const c_char, n: c_int) -> c_int;
    fn winsnstr(w: *mut WINDOW, s: *const c_char, n: c_int) -> c_int;
    fn wgetch(w: *mut WINDOW) -> c_int;
    fn getmaxx(w: *mut WINDOW) -> c_int;
    fn getmaxy(w: *mut WINDOW) -> c_int;
    fn start_color() -> c_int;
    fn assume_default_colors(fg: c_int, bg: c_int) -> c_int;
    fn keypad(w: *mut WINDOW, bf: c_bool) -> c_int;
    fn init_pair(pair: c_short, f: c_short, b: c_short) -> c_int;
    fn wattr_set(w: *mut WINDOW, attrs: attr_t, pair: c_short, opts: *const c_void) -> c_int;
}

trait Checkable where Self: std::marker::Sized {
    fn is_err(&self) -> bool;
    fn check(self) -> Result<Self, ()> {
        if self.is_err() { Err(()) } else { Ok(self) }
    }
}

impl<T> Checkable for *mut T {
    fn is_err(&self) -> bool {
        self.is_null()
    }
}

impl Checkable for c_int {
    fn is_err(&self) -> bool {
        *self == ERR
    }
}

pub struct Scr {
    ptr: *mut WINDOW,
}

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
pub struct Attr: attr_t {
    const NORMAL = 0;
    const STANDOUT = 1 << 16;
    const UNDERLINE = 1 << 17;
    const REVERSE = 1 << 18;
    const BLINK = 1 << 19;
    const DIM = 1 << 20;
    const BOLD = 1 << 21;
    const ALTCHARSET = 1 << 22;
    const INVIS = 1 << 23;
    const PROTECT = 1 << 24;
    const HORIZONTAL = 1 << 25;
    const LEFT = 1 << 26;
    const LOW = 1 << 27;
    const RIGHT = 1 << 28;
    const TOP = 1 << 29;
    const VERTICAL = 1 << 30;
}
}

impl Scr {
    pub fn new() -> Result<Scr, ()> {
        unsafe { setlocale(LC_ALL, "\0".as_ptr() as *const c_char) };
        let p = unsafe { initscr() }.check()?;
        unsafe { start_color() }.check()?;
        for bg in -1 .. 7 {
        for fg in 0 .. 7 {
            if fg == 0 && bg == -1 {
                unsafe { assume_default_colors(0, -1) }
            } else {
                unsafe { init_pair(((1 + bg) << 3) | fg, fg, bg) }
            }.check()?;
        }
        }
        unsafe { noecho() }.check()?;
        unsafe { keypad(p, 1) }.check()?;
        Ok(Scr { ptr: p })
    }
    pub fn get_width(&self) -> Result<c_int, ()> {
        unsafe { getmaxx(self.ptr) }.check()
    }
    pub fn get_height(&self) -> Result<c_int, ()> {
        unsafe { getmaxy(self.ptr) }.check()
    }
    pub fn patch<D, C>(&self, diffs: D) -> Result<(), ()>
        where D : Iterator<Item=(c_int, c_int, C)>
            , C : Iterator<Item=(char, Attr, Color, Option<Color>)> {

        fn color_pair(fg: Color, bg: Option<Color>) -> c_short {
            let bg = match bg {
                Some(c) => 1 + (c as i8 as c_short),
                None => 0
            };
            (bg << 3) | (fg as i8 as c_short)
        }

        let width = self.get_width()?;

        for (y, x, s) in diffs {
            let mut xi = x;
            for (c, attr, fg, bg) in s {
                unsafe { wmove(self.ptr, y, xi) }.check()?;
                unsafe { wattr_set(self.ptr, attr.bits, color_pair(fg, bg), null()) }.check()?;
                let out = if xi + 1 < width { waddnstr } else { winsnstr };
                let mut b = [0; 6];
                let b = c.encode_utf8(&mut b);
                unsafe { out(self.ptr, b.as_bytes().as_ptr() as *const c_char, b.len() as c_int) }.check()?;
                xi = xi + 1;
            }
        }
        Ok(())
    }
    pub fn refresh(&self) -> Result<(), ()> {
        unsafe { wrefresh(self.ptr) }.check()?;
        Ok(())
    }
    pub fn getch(&self) -> Result<Either<c_uint, char>, ()> {
        fn read_u8_tail<G>(b0: u8, g: &G) -> Result<u32, ()> where G : Fn() -> Result<u8, ()> {
            let next = || -> Result<u8, ()> {
                let bi = g()?;
                if bi & 0xC0 != 0x80 { return Err(()); }
                Ok(bi & 0x3F)
            };
            if b0 & 0x80 == 0 { return Ok(b0 as u32); }
            if b0 & 0x40 == 0 { return Err(()); }
            let b1 = next()?;
            if b0 & 0x20 == 0 { return Ok((((b0 & 0x1F) as u32) << 6) | (b1 as u32)); }
            let b2 = next()?;
            if b0 & 0x10 == 0 { return Ok((((b0 & 0x0F) as u32) << 12) | ((b1 as u32) << 6) | (b2 as u32)); }
            let b3 = next()?;
            if b0 & 0x08 == 0 { return Ok((((b0 & 0x07) as u32) << 18) | ((b1 as u32) << 12) | ((b2 as u32) << 6) | (b3 as u32)); }
            let b4 = next()?;
            if b0 & 0x04 == 0 { return Ok((((b0 & 0x03) as u32) << 24) | ((b1 as u32) << 18) | ((b2 as u32) << 12) | ((b3 as u32) << 6) | (b4 as u32)); }
            let b5 = next()?;
            if b0 & 0x02 == 0 { return Ok((((b0 & 0x01) as u32) << 30) | ((b1 as u32) << 24) | ((b2 as u32) << 18) | ((b3 as u32) << 12) | ((b4 as u32) << 6) | (b5 as u32)); }
            Err(())
        }

        let b0 = unsafe { wgetch(self.ptr) }.check()? as c_uint;
        if b0 & KEY_CODE_YES != 0 {
            return Ok(Left(b0));
        }
        let c = read_u8_tail(b0 as u8, &|| {
            let bi = unsafe { wgetch(self.ptr) }.check()? as c_uint;
            if bi & KEY_CODE_YES != 0 { return Err(()); }
            Ok(bi as u8)
        })?;
        match from_u32(c) {
            Some(x) => Ok(Right(x)),
            None => Err(())
        }
    }
}

impl Drop for Scr {
    fn drop(&mut self) {
        unsafe { endwin(); }
    }
}

#[cfg(test)]
mod tests {
    use Scr;
    use Color;
    use Attr;
    use either::{ Left, Right };

    #[test]
    fn it_works() {
        let scr = Scr::new().unwrap();
        scr.patch(Some((6, 133, "ABc".chars().map(|p| (p, Attr::NORMAL, Color::Green, Some(Color::Black))))).into_iter()).unwrap();
        scr.patch(Some((8, 133, Some(('l', Attr::ALTCHARSET | Attr::REVERSE, Color::Green, Some(Color::Black))).into_iter())).into_iter()).unwrap();
        scr.refresh().unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => { scr.patch(Some((6, 2, Some((c, Attr::UNDERLINE, Color::Red, Some(Color::Black))).into_iter())).into_iter()).unwrap(); }
        }
        scr.refresh().unwrap();
        scr.getch().unwrap();
    }
}
