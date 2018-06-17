#![deny(warnings)]
extern crate libc;
extern crate either;

use std::os::raw::{ c_int, c_void };
use libc::{ wchar_t };
use either::{ Either, Left, Right };

include!(concat!(env!("OUT_DIR"), "/ERR.rs"));
include!(concat!(env!("OUT_DIR"), "/chtype.rs"));
include!(concat!(env!("OUT_DIR"), "/wint_t.rs"));
include!(concat!(env!("OUT_DIR"), "/KEY_CODE_YES.rs"));

type WINDOW = c_void;

extern "C" {
    fn initscr() -> *mut WINDOW;
    fn endwin() -> c_int;
    fn wrefresh(w: *mut WINDOW) -> c_int;
    fn wmove(w: *mut WINDOW, y: c_int, x: c_int) -> c_int;
    fn waddchnstr(w: *mut WINDOW, chstr: *const chtype, n: c_int) -> c_int;
    fn wget_wch(w: *mut WINDOW, c: *mut wint_t) -> c_int;
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
        let p = unsafe { initscr() }.check()?;
        Ok(Scr { ptr: p })
    }
    pub fn patch(&self, diffs: &[(c_int, c_int, &[chtype])]) -> Result<(), ()> {
        for &(y, x, s) in diffs {
            unsafe { wmove(self.ptr, y, x) }.check()?;
            unsafe { waddchnstr(self.ptr, s.as_ptr(), s.len() as c_int) }.check()?;
        }
        Ok(())
    }
    pub fn refresh(&self) -> Result<(), ()> {
        unsafe { wrefresh(self.ptr) }.check()?;
        Ok(())
    }
    pub fn getch(&self) -> Result<Either<c_int, wchar_t>, ()> {
        let c: wint_t = 0;
        let r = unsafe { wget_wch(self.ptr, c as *mut wint_t) }.check()?;
        if r == KEY_CODE_YES as wint_t { Ok(Left(c as c_int)) } else { Ok(Right(c as wchar_t)) }
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

    #[test]
    fn it_works() {
        let scr = Scr::new().unwrap();
        scr.patch(&[(6, 1, &[65, 66, 67])]).unwrap();
        scr.refresh().unwrap();
        scr.getch().unwrap();
        scr.patch(&[(6, 2, &[32])]).unwrap();
        scr.refresh().unwrap();
        scr.getch().unwrap();
    }
}
