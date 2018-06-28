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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ValType {
    index: usize
}

impl ValType {
    pub fn box_<T: 'static>(&self, val: T) -> Rc<Val> { Rc::new(Val { type_: *self, unbox: Box::new(val) }) }
    pub fn name(self, fw: &Fw) -> &str {
        fw.val_types[self.index].name()
    }
    pub fn parse(self, s: &str, fw: &Fw) -> Option<Rc<Val>> {
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
    def_val: HashMap<DepProp, Obj>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DepType {
    index: usize
}

fn assert_dep_prop_target(dep_prop: DepProp, dep_type: DepType, fw: &Fw) {
    if !dep_type.is(dep_prop.target(fw), fw) {
        panic!("Dependency property target type mismatch.");
    }
}

fn assert_dep_prop_val(dep_prop: DepProp, val_type: Type, fw: &Fw) {
    if !val_type.is(dep_prop.val_type(fw), fw) {
        panic!("Dependency property value type mismatch.");
    }
}

impl DepType {
    pub fn name(self, fw: &Fw) -> &str {
        &fw.dep_types[self.index].name
    }
    pub fn base(self, fw: &Fw) -> Option<DepType> {
        fw.dep_types[self.index].base
    }
    pub fn def_val(self, dep_prop: DepProp, fw: &Fw) -> &Obj {
        assert_dep_prop_target(dep_prop, self, fw);
        fn self_def_val(dep_type: DepType, fw: &Fw, dep_prop: DepProp) -> Option<&Obj> {
            fw.dep_types[dep_type.index].def_val.get(&dep_prop)
        }
        let mut base = self;
        loop {
            if let Some(ref val) = self_def_val(base, fw, dep_prop) {
                return val;
            }
            if let Some(t) = base.base(fw) { base = t; } else { panic!("DEF_VAL_NOT_FOUND"); }
        }
    }
    pub fn create(self) -> Rc<DepObj> {
        Rc::new(DepObj { type_: self, props: RefCell::new(HashMap::new()) })
    }
    pub fn is(self, dep_type: DepType, fw: &Fw) -> bool {
        let mut base = self;
        loop {
            if base == dep_type { return true; }
            if let Some(t) = base.base(fw) { base = t; } else { return false; }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Type {
    Val(ValType),
    Dep(DepType),
}

impl Type {
    pub fn is(&self, type_: &Type, fw: &Fw) -> bool {
        match self {
            &Type::Val(v) => {
                match type_ {
                    &Type::Val(o_v) => v == o_v,
                    &Type::Dep(_) => false
                }
            },
            &Type::Dep(d) => {
                match type_ {
                    &Type::Val(_) => false,
                    &Type::Dep(o_d) => d.is(o_d, fw)
                }
            }
        }
    }
}

struct DepPropDesc {
    name: String,
    val_type: Type,
    attached: Option<DepType>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DepProp {
    owner: DepType,
    index: usize,
}

impl DepProp {
    pub fn owner(self) -> DepType { self.owner }
    pub fn name(self, fw: &Fw) -> &str {
        &fw.dep_types[self.owner.index].props[self.index].name[..]
    }
    pub fn val_type(self, fw: &Fw) -> &Type {
        &fw.dep_types[self.owner.index].props[self.index].val_type
    }
    pub fn attached(self, fw: &Fw) -> Option<DepType> {
        fw.dep_types[self.owner.index].props[self.index].attached
    }
    pub fn target(self, fw: &Fw) -> DepType {
        self.attached(fw).unwrap_or(self.owner)
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

pub struct GetRef<'a> {
    props: Ref<'a, HashMap<DepProp, Obj>>,
    fw: &'a Fw,
    dep_type: DepType,
    dep_prop: DepProp,
}

impl<'a> Deref for GetRef<'a> {
    type Target = Obj;

    fn deref(&self) -> &Obj {
        self.props.get(&self.dep_prop).unwrap_or_else(|| {
            self.dep_type.def_val(self.dep_prop, self.fw)
        })
    }
}

pub struct GetNonDefRef<'a> {
    props: Ref<'a, HashMap<DepProp, Obj>>,
    dep_prop: DepProp,
}

impl<'a> GetNonDefRef<'a> {
    pub fn borrow(&self) -> Option<&Obj> {
        self.props.get(&self.dep_prop)
    }
}

impl DepObj {
    pub fn type_(&self) -> DepType { self.type_ }
    pub fn get_non_def<'a>(&'a self, dep_prop: DepProp, fw: &Fw) -> GetNonDefRef<'a> {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        GetNonDefRef { props: self.props.borrow(), dep_prop: dep_prop }
    }
    pub fn get<'a>(&'a self, dep_prop: DepProp, fw: &'a Fw) -> GetRef<'a> {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        GetRef { props: self.props.borrow(), fw: fw, dep_prop: dep_prop, dep_type: self.type_ }
    }
    pub fn set(&self, dep_prop: DepProp, val: Obj, fw: &Fw) {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        assert_dep_prop_val(dep_prop, val.type_(), fw);
        self.props.borrow_mut().insert(dep_prop, val);
    }
    pub fn reset(&self, dep_prop: DepProp, fw: &Fw) {
        assert_dep_prop_target(dep_prop, self.type_, fw);
        self.props.borrow_mut().remove(&dep_prop);
    }
    pub fn is(&self, dep_type: DepType, fw: &Fw) -> bool {
        self.type_.is(dep_type, fw)
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
    pub fn type_(&self) -> Type {
        match self {
            &Obj::Val(ref v) => Type::Val(v.type_()),
            &Obj::Dep(ref d) => Type::Dep(d.type_())
        }
    }
    pub fn is(&self, type_: &Type, fw: &Fw) -> bool {
        self.type_().is(type_, fw)
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
        self.dep_types.push(DepTypeDesc { base: base, name: name, props: Vec::new(), props_by_name: HashMap::new(), def_val: HashMap::new() });
        let dep_type = DepType { index: self.dep_types.len() - 1 };
        let name = &self.dep_types[dep_type.index].name;
        match self.dep_types_by_name.entry(name.clone()) {
            Occupied(_) => { panic!("The '{}' dependency type is already registered.", name); }
            Vacant(entry) => entry.insert(dep_type)
        };
        dep_type
    }
    pub fn reg_dep_prop(&mut self, owner: DepType, name: String, val_type: Type, def_val: Obj, attached: Option<DepType>) -> DepProp {
        if !def_val.is(&val_type, self) { panic!("Default value type mismatch."); }
        let dep_prop = {
            let owner_desc = &mut self.dep_types[owner.index];
            owner_desc.props.push(DepPropDesc { name: name, val_type: val_type, attached: attached });
            let dep_prop = DepProp { owner: owner, index: owner_desc.props.len() - 1 };
            let name = &owner_desc.props[dep_prop.index].name;
            match owner_desc.props_by_name.entry(name.clone()) {
                Occupied(_) => { panic!("The '{}' dependency property is already registered for '{}' type.", name, &owner_desc.name); }
                Vacant(entry) => entry.insert(dep_prop)
            };
            dep_prop
        };
        match self.dep_types.get_mut(attached.unwrap_or(owner).index).unwrap().def_val.entry(dep_prop) {
            Occupied(_) => { panic!("DEF_VAL_EXISTS"); }
            Vacant(entry) => entry.insert(def_val)
        };
        dep_prop
    }
    pub fn override_def_val(&mut self, dep_type: DepType, dep_prop: DepProp, def_val: Obj) {
        assert_dep_prop_target(dep_prop, dep_type, self);
        assert_dep_prop_val(dep_prop, def_val.type_(), self);
        match self.dep_types.get_mut(dep_type.index).unwrap().def_val.entry(dep_prop) {
            Occupied(_) => { panic!("Default value is registered already."); }
            Vacant(entry) => entry.insert(def_val)
        };
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
        assert_eq!("123", str_type.parse(&"123", &fw).unwrap().unbox::<String>());
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
        assert_eq!("x", obj.get(name_prop, &fw).unbox::<String>());
        obj.set(name_prop, Obj::Val(str_type.box_(String::from("local value"))), &fw);
        assert_eq!("local value", obj.get(name_prop, &fw).unbox::<String>());
        obj.reset(name_prop, &fw);
        assert_eq!("x", obj.get(name_prop, &fw).unbox::<String>());
        obj.reset(name_prop, &fw);
        assert_eq!("x", obj.get(name_prop, &fw).unbox::<String>());
    }

    #[test]
    fn is_base() {
        let mut fw = Fw::new();
        let base_type = fw.reg_dep_type(String::from("base"), None);
        let obj_type = fw.reg_dep_type(String::from("obj"), Some(base_type));
        assert!(obj_type.is(base_type, &fw));
        assert!(obj_type.is(obj_type, &fw));
        assert!(base_type.is(base_type, &fw));
        assert!(!base_type.is(obj_type, &fw));
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
