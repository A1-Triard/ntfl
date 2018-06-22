#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
extern crate libc;

pub mod scr;
pub mod ncurses;
pub mod window;
pub mod draw;

use std::any::Any;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{ Occupied, Vacant };
use std::fmt::Debug;
use std::ptr;
use std::rc::Rc;

pub trait ValTypeDescr : Debug {
    fn name(&self) -> &str;
    fn parse(&self, type_: ValType, s: &str) -> Option<Rc<Val>>;
    fn to_string(&self, val: &Val) -> String;
}

#[derive(Debug, Copy, Clone)]
pub struct ValType<'a> {
    descr: &'a ValTypeDescr,
}

impl<'a> PartialEq for ValType<'a> {
    fn eq(&self, other: &ValType) -> bool { ptr::eq(self.descr, other.descr) }
}
impl<'a> Eq for ValType<'a> { }

impl<'a> ValType<'a> {
    pub fn name(&self) -> &str { self.descr.name() }
    pub fn parse(&self, s: &str) -> Option<Rc<Val>> { self.descr.parse(*self, s) }
    pub fn box_<T: 'static>(&self, val: T) -> Rc<Val> { Rc::new(Val { type_: *self, unbox: Box::new(val) }) }
}

pub struct Val<'a> {
    type_: ValType<'a>,
    unbox: Box<Any>,
}

impl<'a> Val<'a> {
    pub fn type_(&self) -> ValType { self.type_ }
    pub fn unbox<T: 'static>(&self) -> &T { self.unbox.downcast_ref().unwrap() }
    pub fn to_string(&self) -> String { self.type_.descr.to_string(self) }
}

pub struct Fw<'a> {
    val_types: HashMap<&'a str, Box<ValTypeDescr>>,
}

impl<'a> Fw<'a> {
    pub fn new() -> Fw<'a> {
        Fw { val_types: HashMap::new() }
    }
    pub fn reg_val_type(&mut self, descr: Box<ValTypeDescr>) -> ValType<'a> {
        let name = unsafe { &*(descr.name() as *const str) };
        let ptr = match self.val_types.entry(name) {
            Occupied(_) => { panic!("'{}' value type is already registered.", name); }
            Vacant(entry) => entry.insert(descr)
        }.borrow_mut() as *const ValTypeDescr;
        ValType { descr: unsafe { &*ptr } }
    }
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
