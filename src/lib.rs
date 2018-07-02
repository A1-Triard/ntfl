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
use fw::{ ValType, ValTypeDesc, Fw, Val, DepType, Type, DepProp, Obj };
use window::Rect;

pub struct Size {
    height: isize,
    width: isize,
}

impl Size {
    pub fn new(height: isize, width: isize) -> Size {
        if height < 0 || width < 0 { panic!("Size must be non-negative."); }
        Size { height: height, width: width }
    }
    pub fn height(&self) -> isize { self.height }
    pub fn width(&self) -> isize { self.width }
}

pub struct Ntfl<I> {
    str_type: ValType<I>,
    bool_type: ValType<I>,
    rect_type: ValType<I>,
    visual_type: DepType<I>,
    visual_bounds_prop: DepProp<I>,
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

struct RectTypeDesc { }
impl<I> ValTypeDesc<I> for RectTypeDesc {
    fn name(&self) -> &str { &"Rect" }
    fn parse(&self, type_: ValType<I>, s: &str) -> Option<Arc<Val<I>>> {
        let s = s.trim();
        if s.is_empty() { return Some(type_.box_(Rect::empty())); }
        let mut parts = s.split(',');
        if let Some(part1) = parts.next() {
            if let Some(part2) = parts.next() {
                if let Some(part3) = parts.next() {
                    if let Some(part4) = parts.next() {
                        if parts.next().is_some() { return None; }
                        if let Ok(t) = part1.trim().parse::<isize>() {
                            if let Ok(l) = part2.trim().parse::<isize>() {
                                if let Ok(h) = part3.trim().parse::<isize>() {
                                    if let Ok(w) = part4.trim().parse::<isize>() {
                                        if h <= 0 || w <= 0 { return None; }
                                        return Some(type_.box_(Rect::tlhw(t, l, h, w)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
    fn to_string(&self, val: &Val<I>) -> String {
        let val = val.unbox::<Rect>();
        if let Some((y, x)) = val.loc() {
            let (h, w) = val.size();
            format!("{},{},{},{}", y, x, h, w)
        } else {
            String::new()
        }
    }
}

impl<I> Ntfl<I> {
    pub fn new(fw: &mut Fw<I>) -> Ntfl<I> {
        let str_type = fw.reg_val_type(Box::new(StrTypeDesc { }));
        let bool_type = fw.reg_val_type(Box::new(BoolTypeDesc { }));
        let rect_type = fw.reg_val_type(Box::new(RectTypeDesc { }));
        let visual_type = fw.reg_dep_type(String::from("Visual"), None);
        let visual_bounds_prop = fw.reg_dep_prop(visual_type, String::from("Bounds"), Type::Val(rect_type), Obj::Val(rect_type.box_(Rect::empty())), None);
        Ntfl {
            str_type: str_type,
            bool_type: bool_type,
            rect_type: rect_type,
            visual_type: visual_type,
            visual_bounds_prop: visual_bounds_prop,
        }
    }
    pub fn str_type(&self) -> ValType<I> { self.str_type }
    pub fn bool_type(&self) -> ValType<I> { self.bool_type }
    pub fn rect_type(&self) -> ValType<I> { self.rect_type }
    pub fn visual_type(&self) -> DepType<I> { self.visual_type }
    pub fn visual_bounds_prop(&self) -> DepProp<I> { self.visual_bounds_prop }
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
