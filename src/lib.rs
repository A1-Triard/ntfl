#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
#[macro_use]
extern crate lazy_static;
extern crate libc;

pub mod scr;
pub mod ncurses;
pub mod window;
pub mod draw;
#[macro_use]
pub mod fw;
pub mod inst;

use std::sync::Arc;
use fw::{ ValType, ValTypeDesc, Fw, Val };

pub struct Ntfl<I> {
    str_type: ValType<I>,
    bool_type: ValType<I>,
}

struct StrTypeDesc { }
impl<I> ValTypeDesc<I> for StrTypeDesc {
    fn name(&self) -> &str { &"Str" }
    fn parse(&self, type_: ValType<I>, s: &str) -> Option<Arc<Val<I>>> {
        Some(type_.box_(String::from(s)))
    }
    fn to_string(&self, val: &Val<I>) -> String {
        val.unbox::<String>().clone()
    }
}

struct BoolTypeDesc { }
impl<I> ValTypeDesc<I> for BoolTypeDesc {
    fn name(&self) -> &str { &"Bool" }
    fn parse(&self, type_: ValType<I>, s: &str) -> Option<Arc<Val<I>>> {
        let maybe_val = match s {
            "True" => Some(true),
            "False" => Some(false),
            _ => None
        };
        maybe_val.map(|val| { type_.box_(val) })
    }
    fn to_string(&self, val: &Val<I>) -> String {
        let val = *val.unbox::<bool>();
        String::from(if val { "True" } else { "False" })
    }
}

impl<I> Ntfl<I> {
    pub fn new(fw: &mut Fw<I>) -> Ntfl<I> {
        Ntfl {
            str_type: fw.reg_val_type(Box::new(StrTypeDesc { })),
            bool_type: fw.reg_val_type(Box::new(BoolTypeDesc { })),
        }
    }
    pub fn str_type(&self) -> &ValType<I> { &self.str_type }
    pub fn bool_type(&self) -> &ValType<I> { &self.bool_type }
}

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
        let mut host = WindowsHost::new();
        let height = scr.get_height().unwrap();
        let width = scr.get_width().unwrap();
        let mut bg = host.new_window();
        bg.set_bounds(Rect::tlhw(0, 0, height, width));
        let bg_area = bg.area();
        fill_rect(&mut bg, &bg_area, &' ', Attr::NORMAL, Color::Black, None);
        let mut window = host.new_window();
        window.set_bounds(Rect::tlhw(0, 0, 13, 40));
        let window_area = window.area();
        fill_rect(&mut window, &window_area, &' ', Attr::NORMAL, Color::Black, None);
        draw_border(&mut window, &Rect::tlbr(10, 0, 13, 40), &Border::new().ul(&Graph::LTee).ur(&Graph::RTee), Attr::BOLD, Color::Blue, None);
        draw_border(&mut window, &Rect::tlbr(0, 0, 10, 40), &Border::new().no_bottom(), Attr::BOLD, Color::Blue, None);
        draw_text(&mut window, 1, 1, "Aыcdefgh", Attr::NORMAL, Color::Green, None);
        host.scr(&mut scr);
        scr.refresh(Some((1, 1))).unwrap();
        let mut n = false;
        loop {
            n = !n;
            match scr.getch().unwrap() {
                Left(_) => {
                    let z_index = window.z_index();
                    window.set_z_index(1 - z_index);
                }
                Right('\n') => { break; }
                Right(c) => {
                    fill_rect(&mut bg, &bg_area, &' ', Attr::NORMAL, Color::Black, if n { Some(Color::Green) } else { None });
                    draw_texel(&mut window, 1, 1, &c, Attr::UNDERLINE, Color::Red, None);
                }
            }
            host.scr(&mut scr);
            scr.refresh(None).unwrap();
        }
    }
}
