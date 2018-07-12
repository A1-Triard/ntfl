#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate libc_extra;
extern crate owning_ref;

pub mod scr;
pub mod ncurses;
pub mod window;
pub mod draw;
#[macro_use]
pub mod fw;
pub mod inst;

use std::sync::{ Arc, Mutex };
use either::{ Left, Right };
use ncurses::NCurses;
use scr::{ Scr, Key };
use fw::{ ValType, ValTypeDesc, Fw, Val, DepType, Type, DepProp, Obj, ClassSetLock, DepObj, DepObjDataKey };
use window::{ Rect, WindowsHost, Window };

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

pub struct Ntfl<I> {
    str_type: ValType<I>,
    bool_type: ValType<I>,
    rect_type: ValType<I>,
    visual_type: DepType<I>,
    visual_bounds_prop: DepProp<I>,
    visual_parent_prop: DepProp<I>,
    root_type: DepType<I>,
    root_bounds_lock: ClassSetLock,
    host: Arc<Mutex<WindowsHost>>,
}

impl<I : 'static + Send> Ntfl<I> {
    pub fn new(fw: &mut Fw<I>) -> Ntfl<I> {
        let host = Arc::new(Mutex::new(WindowsHost::new()));
        let str_type = fw.reg_val_type(Box::new(StrTypeDesc { }));
        let bool_type = fw.reg_val_type(Box::new(BoolTypeDesc { }));
        let rect_type = fw.reg_val_type(Box::new(RectTypeDesc { }));
        let visual_window = DepObjDataKey::new();
        let visual_type = {
            let visual_window = visual_window.clone();
            let host = host.clone();
            fw.reg_dep_type(String::from("Visual"), None, Some(Box::new(move |obj, _fw| {
                obj.set_data(visual_window.clone(), Box::new(Mutex::new(host.lock().unwrap().new_window())));
            })))
        };
        let visual_bounds_prop = fw.reg_dep_prop(visual_type, String::from("Bounds"), Type::Val(rect_type), Obj::Val(rect_type.box_(Rect::empty())), None);
        let visual_parent_prop = fw.reg_dep_prop(visual_type, String::from("Parent"), Type::Opt(Box::new(Type::Dep(visual_type))), Obj::Nil(Type::Dep(visual_type)), None);
        let root_type = {
            let visual_window = visual_window.clone();
            fw.reg_dep_type(String::from("Root"), Some(visual_type), Some(Box::new(move |obj, _fw| {
                let window = obj.get_data(&visual_window);
                let mut window = window.borrow().unwrap().downcast_ref::<Mutex<Window>>().unwrap().lock().unwrap();
                window.attach();
            })))
        };
        fw.lock_class_set(root_type, visual_parent_prop);
        let root_bounds_lock = fw.lock_class_set(root_type, visual_bounds_prop);
        {
            let visual_window = visual_window.clone();
            fw.on_changed(visual_type, visual_bounds_prop, Box::new(move |obj, _old, new, _fw| {
                let window = obj.get_data(&visual_window);
                let mut window = window.borrow().unwrap().downcast_ref::<Mutex<Window>>().unwrap().lock().unwrap();
                if window.is_detached() { return; }
                window.set_bounds(new.unbox::<Rect>().clone());
            }));
        }
        {
            let visual_window = visual_window.clone();
            fw.on_changed(visual_type, visual_parent_prop, Box::new(move |obj, _old, new, fw| {
                let window = obj.get_data(&visual_window);
                let mut window = window.borrow().unwrap().downcast_ref::<Mutex<Window>>().unwrap().lock().unwrap();
                if !window.is_detached() { window.detach(); }
                match new {
                    Obj::Nil(_) => { },
                    Obj::Has(parent) => {
                        let parent = parent.dep().get_data(&visual_window);
                        let mut parent = parent.borrow().unwrap().downcast_ref::<Mutex<Window>>().unwrap().lock().unwrap();
                        window.attach_to(&mut parent);
                        window.set_bounds(obj.get(visual_bounds_prop, fw).unbox::<Rect>().clone());
                    },
                    _ => panic!("VISUAL_PARENT")
                };
            }));
        }
        Ntfl {
            str_type: str_type,
            bool_type: bool_type,
            rect_type: rect_type,
            visual_type: visual_type,
            visual_bounds_prop: visual_bounds_prop,
            visual_parent_prop: visual_parent_prop,
            root_type: root_type,
            root_bounds_lock: root_bounds_lock,
            host: host,
        }
    }
    pub fn run(&self, root: &DepObj<I>, fw: &Fw<I>) {
        let mut scr = NCurses::new().unwrap();
        let update_root_bounds = |scr: &Scr| {
            let height = scr.get_height().unwrap();
            let width = scr.get_width().unwrap();
            root.set_locked(self.visual_bounds_prop, Obj::Val(self.rect_type.box_(Rect::tlhw(0, 0, height, width))), &self.root_bounds_lock, fw);
        };
        update_root_bounds(&scr);
        loop {
            self.host.lock().unwrap().scr(&mut scr);
            match scr.getch().unwrap() {
                Left(Key::RESIZE) => {
                    update_root_bounds(&scr);
                },
                Right('q') => {
                    break;
                }
                _ => { }
            }
        }
    }
    pub fn str_type(&self) -> ValType<I> { self.str_type }
    pub fn bool_type(&self) -> ValType<I> { self.bool_type }
    pub fn rect_type(&self) -> ValType<I> { self.rect_type }
    pub fn visual_type(&self) -> DepType<I> { self.visual_type }
    pub fn visual_bounds_prop(&self) -> DepProp<I> { self.visual_bounds_prop }
    pub fn visual_parent_prop(&self) -> DepProp<I> { self.visual_parent_prop }
    pub fn root_type(&self) -> DepType<I> { self.root_type }
}

#[cfg(test)]
mod tests {
    use std::sync::{ Arc, Mutex };
    use fw;

    pub struct TestFw(());
    pub type Fw = fw::Fw<TestFw>;
    //pub type Obj = fw::Obj<TestFw>;
    pub use fw::Obj::Dep as Obj_Dep;
    pub use fw::Obj::Has as Obj_Has;
    pub type Ntfl = ::Ntfl<TestFw>;

    lazy_static! {
        static ref NTFL: (Mutex<Fw>, Ntfl) = {
            let mut fw = Fw::new(TestFw(()));
            let ntfl = Ntfl::new(&mut fw);
            (Mutex::new(fw), ntfl)
        };
    }

    #[test]
    fn create_visual() {
        let fw = &NTFL.0.lock().unwrap();
        let r = NTFL.1.root_type().create(fw);
        let v = NTFL.1.visual_type().create(fw);
        v.set(NTFL.1.visual_parent_prop(), Obj_Has(Arc::new(Obj_Dep(r.clone()))), fw).unwrap();
        NTFL.1.run(&r, fw);
    }
}

#[cfg(test)]
mod functional_tests {
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
        bg.attach();
        bg.set_bounds(Rect::tlhw(0, 0, height, width));
        let bg_area = bg.area();
        fill_rect(&mut bg, &bg_area, &' ', Attr::NORMAL, Color::Black, None);
        let mut window = host.new_window();
        window.attach();
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
