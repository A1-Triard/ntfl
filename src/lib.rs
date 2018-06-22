#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
extern crate libc;

pub mod scr;
pub mod ncurses;
pub mod window;
pub mod draw;

#[cfg(test)]
mod tests {
    use either::{ Left, Right };
    use ncurses::NCurses;
    use scr::{ Scr, Color, Attr };
    use window::{ Rect, WindowsHost };
    use draw::{ draw_border, draw_texel, Border, Graph, draw_text, fill_rect };

    #[test]
    fn it_works() {
        let mut scr = NCurses::new().unwrap();
        let host = WindowsHost::new();
        let height = scr.get_height().unwrap();
        let width = scr.get_width().unwrap();
        let bg = host.new_window();
        bg.set_bounds(Rect::tlhw(0, 0, height, width));
        fill_rect(&bg, &bg.area(), &' ', Attr::NORMAL, Color::Black, None);
        let window = host.new_window();
        window.set_bounds(Rect::tlhw(0, 0, 13, 40));
        fill_rect(&window, &window.area(), &' ', Attr::NORMAL, Color::Black, None);
        draw_border(&window, &Rect::tlbr(10, 0, 13, 40), &Border::new().ul(&Graph::LTee).ur(&Graph::RTee), Attr::BOLD, Color::Blue, None);
        draw_border(&window, &Rect::tlbr(0, 0, 10, 40), &Border::new().no_bottom(), Attr::BOLD, Color::Blue, None);
        draw_text(&window, 1, 1, "Aыcdefgh", Attr::NORMAL, Color::Green, None);
        host.scr(&mut scr);
        scr.refresh(Some((1, 1))).unwrap();
        let mut n = false;
        loop {
            n = !n;
            match scr.getch().unwrap() {
                Left(_) => {
                    window.set_z_index(1 - window.z_index());
                }
                Right('\n') => { break; }
                Right(c) => {
                    fill_rect(&bg, &bg.area(), &' ', Attr::NORMAL, Color::Black, if n { Some(Color::Green) } else { None });
                    draw_texel(&window, 1, 1, &c, Attr::UNDERLINE, Color::Red, None);
                }
            }
            host.scr(&mut scr);
            scr.refresh(None).unwrap();
        }
    }
}
