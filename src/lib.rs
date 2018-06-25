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
use std::cell::{ RefCell, Ref };
use std::collections::HashMap;
use std::collections::hash_map::Entry::{ Occupied, Vacant };
use std::ops::Deref;
use std::rc::Rc;

pub trait ValTypeDesc {
    fn name(&self) -> &str;
    fn parse<'a>(&self, type_: ValType, s: &str) -> Option<Rc<Val>>;
    fn to_string(&self, val: &Val) -> String;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ValType {
    index: usize
}

impl ValType {
    pub fn box_<T: 'static>(&self, val: T) -> Rc<Val> { Rc::new(Val { type_: *self, unbox: Box::new(val) }) }
    pub fn name(self, fw: &Fw) -> &str {
        fw.val_types[self.index].name()
    }
    pub fn parse(self, fw: &Fw, s: &str) -> Option<Rc<Val>> {
        fw.val_types[self.index].parse(self, s)
    }
}

#[derive(Debug)]
pub struct Val {
    type_: ValType,
    unbox: Box<Any>,
}

impl Val {
    pub fn type_(&self) -> ValType { self.type_ }
    pub fn unbox<T: 'static>(&self) -> &T { self.unbox.downcast_ref().unwrap() }
    pub fn to_string(&self, fw: &Fw) -> String {
        fw.val_types[self.type_.index].to_string(self)
    }
}

struct DepTypeDesc {
    base: Option<DepType>,
    name: String,
    props: Vec<DepPropDesc>,
    props_by_name: HashMap<String, DepProp>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DepType {
    index: usize
}

impl DepType {
    pub fn name(self, fw: &Fw) -> &str {
        &fw.dep_types[self.index].name
    }
    pub fn base(self, fw: &Fw) -> Option<DepType> {
        fw.dep_types[self.index].base
    }
    pub fn create(self) -> Rc<DepObj> {
        Rc::new(DepObj { type_: self, props: RefCell::new(HashMap::new()) })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Val(ValType),
    Dep(DepType),
}

struct DepPropDesc {
    name: String,
    val_type: Type,
    attached: Option<DepType>,
    def_val: Obj,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DepProp {
    owner: DepType,
    index: usize,
}

impl DepProp {
    pub fn owner(self) -> DepType { self.owner }
    pub fn name(self, fw: &Fw) -> &str {
        &fw.dep_types[self.owner.index].props[self.index].name[..]
    }
    pub fn def_val(self, fw: &Fw) -> &Obj {
        &fw.dep_types[self.owner.index].props[self.index].def_val
    }
    pub fn val_type(self, fw: &Fw) -> &Type {
        &fw.dep_types[self.owner.index].props[self.index].val_type
    }
    pub fn attached(self, fw: &Fw) -> Option<DepType> {
        fw.dep_types[self.owner.index].props[self.index].attached
    }
}

pub struct Fw {
    val_types: Vec<Box<ValTypeDesc>>,
    val_types_by_name: HashMap<String, ValType>,
    dep_types: Vec<DepTypeDesc>,
    dep_types_by_name: HashMap<String, DepType>,
}

#[derive(Debug)]
pub struct DepObj {
    type_: DepType,
    props: RefCell<HashMap<DepProp, Obj>>,
}

pub struct ObjRef<'a> {
    props: Ref<'a, HashMap<DepProp, Obj>>,
    fw: &'a Fw,
    dep_prop: DepProp,
}

impl<'a> Deref for ObjRef<'a> {
    type Target = Obj;

    fn deref(&self) -> &Obj {
        self.props.get(&self.dep_prop).unwrap_or_else(|| {
            &self.fw.dep_types[self.dep_prop.owner.index].props[self.dep_prop.index].def_val
        })
    }
}

impl DepObj {
    pub fn type_(&self) -> DepType { self.type_ }
    pub fn get<'a>(&'a self, fw: &'a Fw, dep_prop: DepProp) -> ObjRef<'a> {
        ObjRef { props: self.props.borrow(), fw: fw, dep_prop: dep_prop }
    }
    pub fn set(&self, dep_prop: DepProp, val: Obj) {
        self.props.borrow_mut().insert(dep_prop, val);
    }
    pub fn reset(&self, dep_prop: DepProp) {
        self.props.borrow_mut().remove(&dep_prop);
    }
}

#[derive(Debug, Clone)]
pub enum Obj {
    Val(Rc<Val>),
    Dep(Rc<DepObj>),
}

impl Obj {
    pub fn unbox<T: 'static>(&self) -> &T {
        match self {
            Obj::Val(ref v) => v.unbox(),
            Obj::Dep(_) => { panic!("Cannot unbox a dependency object."); }
        }
    }
}

impl Fw {
    pub fn new() -> Fw {
        Fw { val_types: Vec::new(), val_types_by_name: HashMap::new(), dep_types: Vec::new(), dep_types_by_name: HashMap::new() }
    }
    pub fn val_type(&self, name: &str) -> Option<ValType> {
        self.val_types_by_name.get(name).map(|x| { *x })
    }
    pub fn dep_type(&self, name: &str) -> Option<DepType> {
        self.dep_types_by_name.get(name).map(|x| { *x })
    }
    pub fn dep_prop(&self, dep_type: DepType, name: &str) -> Option<DepProp> {
        self.dep_types[dep_type.index].props_by_name.get(name).map(|x| { *x })
    }
    pub fn reg_val_type(&mut self, desc: Box<ValTypeDesc>) -> ValType {
        self.val_types.push(desc);
        let val_type = ValType { index: self.val_types.len() - 1 };
        let name = self.val_types[val_type.index].name();
        match self.val_types_by_name.entry(String::from(name)) {
            Occupied(_) => { panic!("The '{}' value type is already registered.", name); }
            Vacant(entry) => entry.insert(val_type)
        };
        val_type
    }
    pub fn reg_dep_type(&mut self, name: String, base: Option<DepType>) -> DepType {
        self.dep_types.push(DepTypeDesc { base: base, name: name, props: Vec::new(), props_by_name: HashMap::new() });
        let dep_type = DepType { index: self.dep_types.len() - 1 };
        let name = &self.dep_types[dep_type.index].name;
        match self.dep_types_by_name.entry(name.clone()) {
            Occupied(_) => { panic!("The '{}' dependency type is already registered.", name); }
            Vacant(entry) => entry.insert(dep_type)
        };
        dep_type
    }
    pub fn reg_dep_prop(&mut self, owner: DepType, name: String, val_type: Type, def_val: Obj, attached: Option<DepType>) -> DepProp {
        let owner_desc = &mut self.dep_types[owner.index];
        owner_desc.props.push(DepPropDesc { name: name, val_type: val_type, attached: attached, def_val: def_val });
        let dep_prop = DepProp { owner: owner, index: owner_desc.props.len() - 1 };
        let name = &owner_desc.props[dep_prop.index].name;
        match owner_desc.props_by_name.entry(name.clone()) {
            Occupied(_) => { panic!("The '{}' dependency property is already registered for '{}' type.", name, &owner_desc.name); }
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
    use Obj;

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
        assert_eq!("123", str_type.parse(&fw, &"123").unwrap().unbox::<String>());
        assert_eq!("123", str_type.box_(String::from("123")).to_string(&fw));
        assert_eq!("str", str_type.name(&fw));
    }

    #[test]
    fn reg_dep_type_prop_get_set_test() {
        let mut fw = Fw::new();
        let str_type = fw.reg_val_type(Box::new(StrValTypeDesc { }));
        let obj_type = fw.reg_dep_type(String::from("obj"), None);
        let name_prop = fw.reg_dep_prop(obj_type, String::from("name"), Type::Val(str_type), Obj::Val(str_type.box_(String::from("x"))), None);
        assert_eq!("name", name_prop.name(&fw));
        let obj = obj_type.create();
        assert_eq!("x", obj.get(&fw, name_prop).unbox::<String>());
        obj.set(name_prop, Obj::Val(str_type.box_(String::from("local value"))));
        assert_eq!("local value", obj.get(&fw, name_prop).unbox::<String>());
        obj.reset(name_prop);
        assert_eq!("x", obj.get(&fw, name_prop).unbox::<String>());
        obj.reset(name_prop);
        assert_eq!("x", obj.get(&fw, name_prop).unbox::<String>());
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
