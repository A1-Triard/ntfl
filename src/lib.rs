#![deny(warnings)]
extern crate libc;
extern crate either;
#[macro_use]
extern crate bitflags;

pub mod scr;

#[cfg(test)]
mod tests {
    use scr::Scr;
    use scr::Color;
    use scr::Attr;
    use either::{ Left, Right };

    #[test]
    fn it_works() {
        let mut scr = Scr::new().unwrap();
        scr.patch(Some((6, 133, "ABc".chars().map(|p| (p, Attr::NORMAL, Color::Green, Some(Color::Black))))).into_iter()).unwrap();
        scr.patch(Some((8, 133, Some(('l', Attr::ALTCHARSET | Attr::REVERSE, Color::Green, Some(Color::Black))).into_iter())).into_iter()).unwrap();
        scr.refresh(Some((5, 5))).unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => { scr.patch(Some((6, 2, Some((c, Attr::UNDERLINE, Color::Red, Some(Color::Black))).into_iter())).into_iter()).unwrap(); }
        }
        scr.refresh(None).unwrap();
        scr.getch().unwrap();
    }
}
