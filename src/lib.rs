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
use std::collections::HashMap;
use std::collections::hash_map::Entry::{ Occupied, Vacant };
use std::fmt::Debug;
use std::rc::Rc;

pub trait ValTypeDesc : Debug {
    fn name(&self) -> &str;
    fn parse<'a>(&self, type_: ValType, s: &str) -> Option<Rc<Val>>;
    fn to_string(&self, val: &Val) -> String;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ValType {
    index: usize
}

impl ValType {
    pub fn box_<T: 'static>(&self, val: T) -> Rc<Val> { Rc::new(Val { type_: *self, unbox: Box::new(val) }) }
}

pub struct Val {
    type_: ValType,
    unbox: Box<Any>,
}

impl Val {
    pub fn type_(&self) -> ValType { self.type_ }
    pub fn unbox<T: 'static>(&self) -> &T { self.unbox.downcast_ref().unwrap() }
}

#[derive(Debug)]
struct DepTypeDesc {
    base: Option<DepType>,
    name: String,
    props: Vec<DepPropDesc>,
    props_by_name: HashMap<String, DepProp>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DepType {
    index: usize
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Val(ValType),
    Dep(DepType),
}

#[derive(Debug)]
struct DepPropDesc {
    name: String,
    val_type: Type,
    attached: Option<DepType>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DepProp {
    owner: DepType,
    index: usize,
}

impl DepProp {
    pub fn owner(&self) -> DepType { self.owner }
}

pub struct Fw {
    val_types: Vec<Box<ValTypeDesc>>,
    val_types_by_name: HashMap<String, ValType>,
    dep_types: Vec<DepTypeDesc>,
    dep_types_by_name: HashMap<String, DepType>,
}

impl Fw {
    pub fn new() -> Fw {
        Fw { val_types: Vec::new(), val_types_by_name: HashMap::new(), dep_types: Vec::new(), dep_types_by_name: HashMap::new() }
    }
    pub fn val_type(&self, name: &str) -> Option<ValType> {
        self.val_types_by_name.get(name).map(|x| { *x })
    }
    pub fn val_type_name(&self, val_type: ValType) -> &str {
        self.val_types[val_type.index].name()
    }
    pub fn parse(&self, val_type: ValType, s: &str) -> Option<Rc<Val>> {
        self.val_types[val_type.index].parse(val_type, s)
    }
    pub fn to_string(&self, val: &Val) -> String {
        self.val_types[val.type_.index].to_string(val)
    }
    pub fn dep_type(&self, name: &str) -> Option<DepType> {
        self.dep_types_by_name.get(name).map(|x| { *x })
    }
    pub fn dep_type_name(&self, dep_type: DepType) -> &str {
        &self.dep_types[dep_type.index].name
    }
    pub fn base(&self, dep_type: DepType) -> Option<DepType> {
        self.dep_types[dep_type.index].base
    }
    pub fn dep_prop_name(&self, dep_prop: DepProp) -> &str {
        &self.dep_types[dep_prop.owner.index].props[dep_prop.index].name[..]
    }
    pub fn dep_prop_val_type(&self, dep_prop: DepProp) -> &Type {
        &self.dep_types[dep_prop.owner.index].props[dep_prop.index].val_type
    }
    pub fn dep_prop_attached(&self, dep_prop: DepProp) -> Option<DepType> {
        self.dep_types[dep_prop.owner.index].props[dep_prop.index].attached
    }
    pub fn reg_val_type(&mut self, desc: Box<ValTypeDesc>) -> ValType {
        self.val_types.push(desc);
        let val_type = ValType { index: self.val_types.len() - 1 };
        let name = self.val_types[val_type.index].name();
        match self.val_types_by_name.entry(String::from(name)) {
            Occupied(_) => { panic!("'{}' value type is already registered.", name); }
            Vacant(entry) => entry.insert(val_type)
        };
        val_type
    }
    pub fn reg_dep_type(&mut self, name: String, base: Option<DepType>) -> DepType {
        self.dep_types.push(DepTypeDesc { base: base, name: name, props: Vec::new(), props_by_name: HashMap::new() });
        let dep_type = DepType { index: self.dep_types.len() - 1 };
        let name = &self.dep_types[dep_type.index].name;
        match self.dep_types_by_name.entry(name.clone()) {
            Occupied(_) => { panic!("'{}' dependency type is already registered.", name); }
            Vacant(entry) => entry.insert(dep_type)
        };
        dep_type
    }
    pub fn reg_prop(&mut self, owner: DepType, name: String, val_type: Type, attached: Option<DepType>) -> DepProp {
        let owner_desc = &mut self.dep_types[owner.index];
        owner_desc.props.push(DepPropDesc { name: name, val_type: val_type, attached: attached });
        let dep_prop = DepProp { owner: owner, index: owner_desc.props.len() - 1 };
        let name = &owner_desc.props[dep_prop.index].name;
        match owner_desc.props_by_name.entry(name.clone()) {
            Occupied(_) => { panic!("'{}' dependency property is already registered for '{}' type.", name, &owner_desc.name); }
            Vacant(entry) => entry.insert(dep_prop)
        };
        dep_prop
    }
}

#[cfg(test)]
mod tests {
    use either::{ Left, Right };
    use ncurses::NCurses;
    use scr::{ Scr, Color, Attr };
    use window::{ Rect, WindowsHost };
    use draw::{ draw_border, draw_texel, Border, Graph, draw_text, fill_rect };

    use std::rc::Rc;
    use ValTypeDesc;
    use ValType;
    use Val;
    use Fw;
    use Type;

    #[derive(Debug)]
    struct StrValTypeDesc { }
    impl ValTypeDesc for StrValTypeDesc {
        fn name(&self) -> &str { &"str" }
        fn parse(&self, type_: ValType, s: &str) -> Option<Rc<Val>> {
            Some(type_.box_(String::from(s)))
        }
        fn to_string(&self, val: &Val) -> String { val.unbox::<String>().clone() }
    }

    #[test]
    fn reg_val_type_test() {
        let mut fw = Fw::new();
        let str_type = fw.reg_val_type(Box::new(StrValTypeDesc { }));
        assert_eq!("123", fw.parse(str_type, &"123").unwrap().unbox::<String>());
        assert_eq!("123", fw.to_string(&str_type.box_(String::from("123"))));
        assert_eq!("str", fw.val_type_name(str_type));
    }

    #[test]
    fn reg_dep_type_prop_test() {
        let mut fw = Fw::new();
        let str_type = fw.reg_val_type(Box::new(StrValTypeDesc { }));
        let obj_type = fw.reg_dep_type(String::from("obj"), None);
        let name_prop = fw.reg_prop(obj_type, String::from("name"), Type::Val(str_type), None);
        assert_eq!("name", fw.dep_prop_name(name_prop));
    }

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
