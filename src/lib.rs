#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
extern crate libc;

pub mod scr;
pub mod ncurses;
pub mod window;

#[cfg(test)]
mod tests {
    use scr::Scr;
    use scr::Color;
    use scr::Attr;
    use scr::Texel;
    use ncurses::NCurses;
    use either::{ Left, Right };

    #[test]
    fn it_works() {
        let mut scr = NCurses::new().unwrap();
        scr.out(6, 133, &Texel { ch: 'A', attr: Attr::NORMAL, fg: Color::Green, bg: None }).unwrap();
        scr.out(6, 134, &Texel { ch: 'B', attr: Attr::NORMAL, fg: Color::Green, bg: None }).unwrap();
        scr.out(6, 135, &Texel { ch: 'c', attr: Attr::NORMAL, fg: Color::Green, bg: None }).unwrap();
        scr.out(5, 5, &Texel { ch: 'l', attr: Attr::ALTCHARSET | Attr::REVERSE, fg: Color::Green, bg: Some(Color::Black) }).unwrap();
        scr.refresh(Some((5, 5))).unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => { scr.out(6, 2, &Texel { ch: c, attr: Attr::UNDERLINE, fg: Color::Red, bg: None }).unwrap(); }
        }
        scr.refresh(None).unwrap();
        scr.getch().unwrap();
    }
}
