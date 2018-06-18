#![deny(warnings)]
extern crate libc;
extern crate either;

use std::char::from_u32;
use std::os::raw::{ c_int, c_void, c_short, c_char, c_uint };
use std::ptr::null;
use either::{ Either, Left, Right };
use libc::{ setlocale, LC_ALL };

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
    fn wchgat(w: *mut WINDOW, n: c_int, attr: attr_t, pair: c_short, opts: *const c_void) -> c_int;
    fn waddnstr(w: *mut WINDOW, s: *const c_char, n: c_int) -> c_int;
    fn wgetch(w: *mut WINDOW) -> c_int;
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

impl Scr {
    pub fn new() -> Result<Scr, ()> {
        unsafe { setlocale(LC_ALL, "\0".as_ptr() as *const c_char) };
        let p = unsafe { initscr() }.check()?;
        unsafe { noecho() }.check()?;
        Ok(Scr { ptr: p })
    }
    pub fn patch<D, C>(&self, diffs: D) -> Result<(), ()>
        where D : Iterator<Item=(c_int, c_int, C)>
            , C : Iterator<Item=(char, attr_t, c_int)> {

        for (y, x, s) in diffs {
            let mut xi = x;
            for (c, attr, pair) in s {
                unsafe { wmove(self.ptr, y, xi) }.check()?;
                unsafe { wchgat(self.ptr, 1, attr, pair as c_short, null()) }.check()?;
                let mut b = [0; 6];
                let b = c.encode_utf8(&mut b);
                unsafe { waddnstr(self.ptr, b.as_bytes().as_ptr() as *const c_char, b.len() as c_int) }.check()?;
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
    use either::{ Left, Right };

    #[test]
    fn it_works() {
        let scr = Scr::new().unwrap();
        scr.patch(Some((6, 1, "ABc".chars().map(|p| (p, 0, 0)))).into_iter()).unwrap();
        scr.refresh().unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => { scr.patch(Some((6, 2, Some((c, 0, 0)).into_iter())).into_iter()).unwrap(); }
        }
        scr.refresh().unwrap();
        scr.getch().unwrap();
    }
}
