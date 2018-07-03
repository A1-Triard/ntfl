#![deny(warnings)]
use std::char::from_u32;
use std::marker::Sized;
use std::os::raw::{ c_int, c_void, c_short, c_char, c_uint };
use std::ptr::null;
use either::{ Either, Left, Right };
use libc::{ setlocale, LC_ALL };

use scr::{ Color, Scr, Texel, Key };

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
    fn curs_set(visibility: c_int) -> c_int;
}

trait Checkable where Self: Sized {
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

pub struct NCurses {
    ptr: *mut WINDOW,
    cursor_is_visible: bool,
}

impl NCurses {
    pub fn new() -> Result<NCurses, ()> {
        unsafe { setlocale(LC_ALL, "\0".as_ptr() as *const c_char) };
        let p = unsafe { initscr() }.check()?;
        unsafe { start_color() }.check()?;
        for bg in -1 .. 8 {
        for fg in 0 .. 8 {
            if fg == 0 && bg == -1 {
                unsafe { assume_default_colors(0, -1) }
            } else {
                unsafe { init_pair(((1 + bg) << 3) | fg, fg, bg) }
            }.check()?;
        }
        }
        unsafe { noecho() }.check()?;
        unsafe { keypad(p, 1) }.check()?;
        unsafe { curs_set(0) };
        Ok(NCurses { ptr: p, cursor_is_visible: false })
    }
    fn get_width_i(&self) -> Result<c_int, ()> {
        unsafe { getmaxx(self.ptr) }.check()
    }
    fn get_height_i(&self) -> Result<c_int, ()> {
        unsafe { getmaxy(self.ptr) }.check()
    }
}

impl Scr for NCurses {
    fn get_width(&self) -> Result<isize, ()> {
        let w = self.get_width_i()?;
        Ok(w as isize)
    }
    fn get_height(&self) -> Result<isize, ()> {
        let h = self.get_height_i()?;
        Ok(h as isize)
    }
    fn out(&mut self, y: isize, x: isize, c: &Texel) -> Result<(), ()> {
        fn color_pair(fg: Color, bg: Option<Color>) -> c_short {
            let bg = match bg {
                Some(c) => 1 + (c as i8 as c_short),
                None => 0
            };
            (bg << 3) | (fg as i8 as c_short)
        }

        let y = y as c_int;
        let x = x as c_int;
        unsafe { wmove(self.ptr, y, x) }.check()?;
        unsafe { wattr_set(self.ptr, (c.attr.bits() as attr_t) << 16, color_pair(c.fg, c.bg), null()) }.check()?;
        let outstr = if x + 1 < self.get_width_i()? { waddnstr } else { winsnstr };
        let mut b = [0; 6];
        let b = c.ch.encode_utf8(&mut b);
        unsafe { outstr(self.ptr, b.as_bytes().as_ptr() as *const c_char, b.len() as c_int) }.check()?;
        Ok(())
    }
    fn refresh(&mut self, cursor: Option<(isize, isize)>) -> Result<(), ()> {
        match cursor {
            None => {
                if self.cursor_is_visible {
                    unsafe { curs_set(0); }
                    self.cursor_is_visible = false;
                }
            },
            Some((y, x)) => {
                let y = y as c_int;
                let x = x as c_int;
                if !self.cursor_is_visible {
                    unsafe { curs_set(1); }
                    self.cursor_is_visible = true;
                }
                unsafe { wmove(self.ptr, y, x) }.check()?;
            }
        }
        unsafe { wrefresh(self.ptr) }.check()?;
        Ok(())
    }
    fn getch(&mut self) -> Result<Either<Key, char>, ()> {
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
            return Ok(Left(Key { value: b0 as u32 }));
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

impl Drop for NCurses {
    fn drop(&mut self) {
        unsafe { endwin(); }
    }
}
