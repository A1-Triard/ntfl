#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
extern crate libc;

use std::cell::RefCell;
use std::cmp::{ min, max };
use std::mem::replace;
use std::ops::DerefMut;
use std::rc::Rc;

pub mod scr;
pub mod ncurses;
use scr::{ Attr, Color, Scr, Texel };

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
struct RectValue {
    top: isize,
    left: isize,
    height: isize,
    width: isize,
}

impl RectValue {
    pub fn bottom(&self) -> isize { self.top + self.height }
    pub fn right(&self) -> isize { self.left + self.width }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct Rect {
    val: Option<RectValue>,
}

impl Rect {
    pub fn empty() -> Rect { Rect { val: None } }
    pub fn tlhw(top: isize, left: isize, height: isize, width: isize) -> Rect {
        Rect {
            val: if height <= 0 || width <= 0 {
                None
            } else {
                Some(RectValue { top: top, left: left, height: height, width: width })
            }
        }
    }
    pub fn tlbr(top: isize, left: isize, bottom: isize, right: isize) -> Rect {
        Rect {
            val: if bottom <= top || right <= left {
                None
            } else {
                Some(RectValue { top: top, left: left, height: bottom - top, width: right - left })
            }
        }
    }
    pub fn clear(&mut self) {
        self.val = None;
    }
    pub fn loc(&self) -> Option<(isize, isize)> {
        match self.val {
            None => None,
            Some(ref val) => Some((val.top, val.left))
        }
    }
    pub fn size(&self) -> (isize, isize) {
        match self.val {
            None => (0, 0),
            Some(ref val) => (val.height, val.width)
        }
    }
    pub fn contains(&self, y: isize, x: isize) -> bool {
        match self.val {
            None => false,
            Some(ref val) => y >= val.top && x >= val.left && y < val.bottom() && x < val.right()
        }
    }
    pub fn include(&mut self, y: isize, x: isize) {
        if let Some(ref mut val) = self.val {
            if y <= val.top { val.top = y; } else { val.height = max(val.height, y + 1 - val.top); }
            if x <= val.left { val.left = x; } else { val.width = max(val.width, x + 1 - val.left); }
        } else {
            self.val = Some(RectValue { top: x, left: y, height: 1, width: 1 });
        }
    }
    pub fn offset(&mut self, dy: isize, dx: isize) {
        if let Some(ref mut val) = self.val {
            val.top += dy;
            val.left += dx;
        }
    }
    pub fn union(&mut self, r: Rect) {
        if let Some(r) = r.val {
            if let Some(ref mut val) = self.val {
                let bottom = val.bottom();
                let right = val.right();
                val.top = min(val.top, r.top);
                val.left = min(val.left, r.left);
                val.height = max(bottom, r.bottom()) - val.top;
                val.width = max(right, r.right()) - val.left;
            } else {
                self.val = Some(r);
            }
        }
    }
    pub fn intersection(&self, r: &Rect) -> Rect {
        if let Some(ref val) = self.val {
            if let Some(ref r) = r.val {
                Rect::tlbr(
                    max(val.top, r.top),
                    max(val.left, r.left),
                    min(val.bottom(), r.bottom()),
                    min(val.right(), r.right())
                )
            } else {
                Rect::empty()
            }
        } else {
            Rect::empty()
        }
    }
    pub fn scan<I, R>(&self, mut it: I) -> Option<R> where I : FnMut(isize, isize) -> Option<R> {
        if let Some(ref val) = self.val {
            for y in val.top .. val.bottom() {
                for x in val.left .. val.right() {
                    if let Some(r) = it(y, x) { return Some(r); }
                }
            }
        }
        None
    }
}

struct WindowData {
    bounds: Rect,
    content: Vec<Vec<Texel>>,
    invalid: Rect,
    parent: Option<Rc<RefCell<WindowData>>>,
    subwindows: Vec<Rc<RefCell<WindowData>>>,
}

impl WindowData {
    fn new(parent: Option<Rc<RefCell<WindowData>>>) -> WindowData {
        WindowData {
            bounds: Rect::empty(),
            content: Vec::new(),
            invalid: Rect::empty(),
            parent: parent,
            subwindows: Vec::new()
        }
    }
    fn set_bounds(&mut self, bounds: Rect) -> Rect {
        let (height, width) = bounds.size();
        for row in &mut self.content {
            row.resize(width as usize, Texel { ch: 'X', attr: Attr::BOLD, fg: Color::Red, bg: None });
        }
        self.content.resize(height as usize, vec![Texel { ch: 'X', attr: Attr::BOLD, fg: Color::Red, bg: None }; width as usize]);
        self.invalid = self.invalid.intersection(&bounds);
        replace(&mut self.bounds, bounds)
    }
    fn global_bounds(&self) -> Rect {
        fn global_offset(window: &WindowData, rect: &mut Rect) {
            match window.bounds.loc() {
                None => { rect.clear(); return; }
                Some((y, x)) => { rect.offset(y, x); }
            }
            if let Some(ref parent) = window.parent {
                global_offset(&parent.borrow(), rect);
            }
        }
        let mut bounds = *&self.bounds;
        if let Some(ref parent) = self.parent {
            global_offset(&parent.borrow(), &mut bounds);
        }
        bounds
    }
    fn out(&mut self, y: isize, x: isize, c: Texel) {
        self.invalid.include(y, x);
        replace(&mut self.content[y as usize][x as usize], c);
    }
    fn scr(&mut self, s: &mut Scr, global_invalid: &mut Rect) {
        let mut invalid = replace(&mut self.invalid, Rect::empty());
        if let Some((y, x)) = self.bounds.loc() {
            invalid.offset(y, x);
            global_invalid.union(invalid);
            let mut viewport = self.bounds.intersection(global_invalid);
            viewport.offset(-y, -x);
            let err = viewport.scan(|yi, xi| {
                let texel = &self.content[yi as usize][xi as usize];
                match s.out(y + yi, x + xi, texel) {
                    Err(()) => Some(()),
                    Ok(()) => None
                }
            });
            if let Some(()) = err {
                eprintln!("NTFL render error occuried!");
            }
        }
    }
}

pub struct Window {
    host: Rc<RefCell<WindowsHostValue>>,
    data: Rc<RefCell<WindowData>>,
}

struct WindowsHostValue {
    windows: Vec<Rc<RefCell<WindowData>>>,
    invalid: Rect,
}

pub struct WindowsHost {
    val: Rc<RefCell<WindowsHostValue>>,
}

impl WindowsHost {
    pub fn new() -> WindowsHost {
        WindowsHost { val: Rc::new(RefCell::new(WindowsHostValue { windows: Vec::new(), invalid: Rect::empty() })) }
    }
    pub fn new_window(&self) -> Window {
        let w = Rc::new(RefCell::new(WindowData::new(None)));
        self.val.borrow_mut().windows.push(Rc::clone(&w));
        Window { host: Rc::clone(&self.val), data: w }
    }
    pub fn scr(&self, s: &mut Scr) {
        fn scr_window(window: &mut WindowData, s: &mut Scr, invalid: &mut Rect) {
            window.scr(s, invalid);
            for subwindow in window.subwindows.iter_mut() {
                scr_window(subwindow.borrow_mut().deref_mut(), s, invalid);
            }
        }
        let mut ref_mut = self.val.borrow_mut();
        let ref mut b = ref_mut.deref_mut();
        let mut invalid = replace(&mut b.invalid, Rect::empty());
        for w in b.windows.iter_mut() {
            scr_window(&mut w.borrow_mut(), s, &mut invalid);
        }
    }
}

impl Window {
    pub fn out(&self, y: isize, x: isize, c: Texel) {
        self.data.borrow_mut().out(y, x, c);
    }
    pub fn set_bounds(&self, bounds: Rect) {
        self.host.borrow_mut().invalid.union(self.data.borrow().global_bounds());
        self.data.borrow_mut().set_bounds(bounds);
        self.host.borrow_mut().invalid.union(self.data.borrow().global_bounds());
    }
    pub fn new_sub(&self) -> Window {
        let w = Rc::new(RefCell::new(WindowData::new(Some(Rc::clone(&self.data)))));
        self.data.borrow_mut().subwindows.push(Rc::clone(&w));
        Window { host: Rc::clone(&self.host), data: w }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        fn del_window(windows: &mut Vec<Rc<RefCell<WindowData>>>, window: &Rc<RefCell<WindowData>>) {
            let i = windows.iter().enumerate().filter(|(_, w)| { Rc::ptr_eq(w, window) }).next().unwrap().0;
            windows.remove(i);
        }
        self.host.borrow_mut().invalid.union(self.data.borrow_mut().set_bounds(Rect::empty()));
        if let Some(ref parent) = self.data.borrow().parent {
            del_window(&mut parent.borrow_mut().subwindows, &self.data);
        } else {
            del_window(&mut self.host.borrow_mut().windows, &self.data);
        };
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use Rect;
    use WindowData;
    use WindowsHost;
    use scr::tests::TestScr;

    use scr::Scr;
    use scr::Color;
    use scr::Attr;
    use scr::Texel;
    use ncurses::NCurses;
    use either::{ Left, Right };

    #[test]
    fn rect_contains() {
        let r = Rect::tlhw(5, 7, 10, 70);
        assert!(r.contains(10, 10));
        assert!(r.contains(5, 10));
        assert!(!r.contains(10, 5));
    }

    #[test]
    fn window_scr() {
        let mut w = WindowData::new(None);
        w.set_bounds(Rect::tlhw(3, 5, 1, 2));
        assert_eq!(Rect::empty(), w.invalid);
        w.out(0, 0, Texel { ch: '+', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        w.out(0, 1, Texel { ch: '-', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        let mut s = TestScr::new(100, 100);
        let mut invalid = Rect::empty();
        w.scr(&mut s, &mut invalid);
        assert!(Some((3, 5)) == invalid.loc());
        assert!((1, 2) == invalid.size(), format!("({}, {})", invalid.size().0, invalid.size().1));
        assert!('+' == s.content(3, 5).ch, format!("{}", s.content(3, 5).ch));
        assert!('-' == s.content(3, 6).ch, format!("{}", s.content(3, 6).ch));
    }

    #[test]
    fn window_set_bounds() {
        let mut w = WindowData::new(None);
        w.set_bounds(Rect::tlhw(5, 7, 3, 500));
        assert!(Rect::tlhw(5, 7, 3, 500) == w.bounds);
    }

    #[test]
    fn new_window_drop() {
        let host = WindowsHost::new();
        let window_ref = {
            let mut window = host.new_window();
            let window_ref = Rc::downgrade(&window.data);
            assert!(window_ref.upgrade().is_some());
            window_ref
        };
        if let Some(window) = window_ref.upgrade() {
            panic!(format!("{}", Rc::strong_count(&window)));
        }
    }

    #[test]
    fn windows_host_scr_works() {
        let mut s = TestScr::new(100, 100);
        let host = WindowsHost::new();
        let window = host.new_window();
        window.set_bounds(Rect::tlhw(3, 5, 1, 2));
        host.scr(&mut s);
    }

    #[test]
    fn windows_host_scr() {
        let mut s = TestScr::new(100, 100);
        let host = WindowsHost::new();
        let window = host.new_window();
        window.set_bounds(Rect::tlhw(3, 5, 1, 2));
        window.out(0, 0, Texel { ch: '+', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        window.out(0, 1, Texel { ch: '-', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        host.scr(&mut s);
        assert!('+' == s.content(3, 5).ch, format!("{}", s.content(3, 5).ch));
        assert!('-' == s.content(3, 6).ch, format!("{}", s.content(3, 6).ch));
    }

    #[test]
    fn subwindow_bounds() {
        let host = WindowsHost::new();
        let window = host.new_window();
        let sub = window.new_sub();
        sub.set_bounds(Rect::tlhw(1, 1, 1, 1));
        assert_eq!(Rect::empty(), host.val.borrow().invalid);
    }
    //#[test]
    //fn windows_hierarhy() {
        //let host = WindowsHost::new();
        //let window1 = host.new_window();
        //let window2 = host.new_window();
        //let sub1 = window1.new_sub();
        //let sub2 = window2.new_sub();
        //let sub3 = window2.new_sub();
        //window1.set_bounds(Rect::tlhw(
    //}

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
