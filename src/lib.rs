#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
extern crate libc;

pub mod scr;
pub mod ncurses;
pub mod window;

use scr::{ Color, Attr, Texel };
use window::Window;

pub fn draw_h_line(window: &Window, y: isize, x1: isize, x2: isize, attr: Attr, fg: Color, bg: Option<Color>) {
    if let Some((x1, x2)) = window.area().inters_h_line(y, x1, x2) {
        for x in x1 .. x2 {
            window.out(y, x, Texel { ch: 'q', attr: attr | Attr::ALTCHARSET, fg: fg, bg: bg });
        }
    }
}

#[cfg(test)]
mod tests {
    use either::{ Left, Right };
    use ncurses::NCurses;
    use scr::{ Scr, Color, Attr, Texel };
    use window::{ Rect, WindowsHost };
    use draw_h_line;

    #[test]
    fn it_works() {
        let mut scr = NCurses::new().unwrap();
        let host = WindowsHost::new();
        let window = host.new_window();
        let height = scr.get_height().unwrap();
        let width = scr.get_width().unwrap();
        window.set_bounds(Rect::tlhw(0, 0, height, width));
        draw_h_line(&window, 3, 0, 20, Attr::NORMAL, Color::Blue, None);
        window.out(6, 133, Texel { ch: 'A', attr: Attr::NORMAL, fg: Color::Green, bg: None });
        window.out(6, 134, Texel { ch: 'B', attr: Attr::NORMAL, fg: Color::Green, bg: None });
        window.out(6, 135, Texel { ch: 'c', attr: Attr::NORMAL, fg: Color::Green, bg: None });
        window.out(5, 5, Texel { ch: 'l', attr: Attr::ALTCHARSET | Attr::REVERSE, fg: Color::Green, bg: Some(Color::Black) });
        host.scr(&mut scr);
        scr.refresh(Some((5, 5))).unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => {
                window.out(6, 2, Texel { ch: c, attr: Attr::UNDERLINE, fg: Color::Red, bg: None });
                host.scr(&mut scr);
                scr.refresh(None).unwrap();
                scr.getch().unwrap();
            }
        }
    }
}
