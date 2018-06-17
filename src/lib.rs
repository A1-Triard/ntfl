include!(concat!(env!("OUT_DIR"), "/chtype.rs"));

use std::os::raw::{ c_int };

//use curses_sys::{ WINDOW, chtype, initscr, endwin, waddchnstr, wmove, wrefresh, wgetch };

//fn p_check<T>(p: *mut T) -> Result<*mut T, ()> {
    //if p.is_null() { Err(())} else { Ok(p) }
//}

//fn check(i: c_int) -> Result<(), ()> {
    //if i == -1 { Err(()) } else { Ok(()) }
//}

//struct Scr {
    //ptr: *mut WINDOW,
//}

//impl Scr {
    //fn new() -> Result<Scr, ()> {
        //let p = p_check(unsafe { initscr() })?;
        //Ok(Scr { ptr: p })
    //}
    //fn patch(&self, diffs: &[(c_int, c_int, &[chtype])]) -> Result<(), ()> {
        //for &(y, x, s) in diffs {
            //check(unsafe { wmove(self.ptr, y, x) })?;
            //check(unsafe { waddchnstr(self.ptr, s.as_ptr(), s.len() as c_int) })?;

        //}
        //Ok(())
    //}
    //fn refresh(&self) -> Result<(), ()> {
        //check(unsafe { wrefresh(self.ptr) })
    //}
    //fn getch(&self) -> Option<c_int> {
        //let c = unsafe { wgetch(self.ptr) };
        //if c == -1 { None } else { Some(c) }
    //}
//}

//impl Drop for Scr {
    //fn drop(&mut self) {
        //unsafe { endwin(); }
    //}
//}



#[cfg(test)]
mod tests {
//    use Scr;

    #[test]
    fn it_works() {
        //let scr = Scr::new().unwrap();
        //scr.patch(&[(6, 1, &[65, 66, 67])]).unwrap();
        //scr.refresh().unwrap();
        //scr.getch();
    }
}
