#![deny(warnings)]

use std::cell::{ Ref, RefCell };
use std::cmp::{ min, max };
use std::mem::replace;
use std::ops::{ Deref, DerefMut };
use std::rc::Rc;

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
            self.val = Some(RectValue { top: y, left: x, height: 1, width: 1 });
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
    pub fn inters_rect(&self, r: &Rect) -> Rect {
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
    pub fn inters_h_line(&self, y: isize, x1: isize, x2: isize) -> Option<(isize, isize)> {
        self.val.as_ref().and_then(|v| {
            if y < v.top || y >= v.bottom() { return None; }
            let x1 = max(x1, v.left);
            let x2 = min(x2, v.right());
            if x1 >= x2 { None } else { Some((x1, x2)) }
        })
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
        self.invalid = self.invalid.inters_rect(&Rect::tlhw(0, 0, height, width));
        replace(&mut self.bounds, bounds)
    }
    fn out(&mut self, y: isize, x: isize, c: Texel) {
        self.invalid.include(y, x);
        replace(&mut self.content[y as usize][x as usize], c);
    }
    fn scr(&mut self, s: &mut Scr, parent_y: isize, parent_x: isize, crop_height: isize, crop_width: isize, global_invalid: &mut Rect) -> Rect {
        let mut invalid = replace(&mut self.invalid, Rect::empty());
        match self.bounds.loc() {
            None => Rect::empty(),
            Some((y, x)) => {
                let mut bounds = *&self.bounds;
                bounds.offset(parent_y, parent_x);
                let viewport = bounds.inters_rect(&Rect::tlhw(parent_y, parent_x, crop_height, crop_width));
                let y0 = parent_y + y;
                let x0 = parent_x + x;
                invalid.offset(y0, x0);
                global_invalid.union(invalid.inters_rect(&viewport));
                let err = viewport.inters_rect(global_invalid).scan(|yi, xi| {
                    let texel = &self.content[(yi - y0) as usize][(xi - x0) as usize];
                    match s.out(yi, xi, texel) {
                        Err(()) => Some(()),
                        Ok(()) => None
                    }
                });
                if let Some(()) = err {
                    #[cfg(test)]
                    panic!("NTFL render error occuried!");
                    #[cfg(not(test))]
                    eprintln!("NTFL render error occuried!");
                }
                viewport
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
        fn scr_window(window: &mut WindowData, s: &mut Scr, parent_y: isize, parent_x: isize, crop_height: isize, crop_width: isize, invalid: &mut Rect) {
            let viewport = window.scr(s, parent_y, parent_x, crop_height, crop_width, invalid);
            if let Some((y, x)) = viewport.loc() {
                let (height, width) = viewport.size();
                for subwindow in window.subwindows.iter_mut() {
                    scr_window(subwindow.borrow_mut().deref_mut(), s, y, x, height, width, invalid);
                }
            }
        }
        let mut ref_mut = self.val.borrow_mut();
        let ref mut b = ref_mut.deref_mut();
        let mut invalid = replace(&mut b.invalid, Rect::empty());
        let height = s.get_height().unwrap();
        let width = s.get_width().unwrap();
        for w in b.windows.iter_mut() {
            scr_window(&mut w.borrow_mut(), s, 0, 0, height, width, &mut invalid);
        }
    }
}

pub struct WindowBoundsRef<'a> {
    data: Ref<'a, WindowData>
}

impl<'a> Deref for WindowBoundsRef<'a> {
    type Target = Rect;

    fn deref(&self) -> &Rect {
        &self.data.bounds
    }
}

impl Window {
    pub fn out(&self, y: isize, x: isize, c: Texel) {
        self.data.borrow_mut().out(y, x, c);
    }
    pub fn bounds(&self) -> WindowBoundsRef {
        WindowBoundsRef { data: self.data.borrow() }
    }
    pub fn area(&self) -> Rect {
        let (height, width) = self.data.borrow().bounds.size();
        Rect::tlhw(0, 0, height, width)
    }
    pub fn set_bounds(&self, bounds: Rect) {
        fn global(window: &WindowData, child_y: isize, child_x: isize) -> Option<(isize, isize)> {
            window.bounds.loc()
                .map(|(y, x)| (y + child_y, x + child_x))
                .and_then(|(y, x)| window.parent.as_ref().map_or(Some((y, x)), |parent| global(&parent.borrow(), y, x)))
        }
        let mut new_bounds = *&bounds;
        let mut old_bounds = self.data.borrow_mut().set_bounds(bounds);
        if let Some((parent_y, parent_x)) = self.data.borrow().parent.as_ref().map_or(Some((0, 0)), |parent| global(&parent.borrow(), 0, 0)) {
            old_bounds.offset(parent_y, parent_x);
            new_bounds.offset(parent_y, parent_x);
            self.host.borrow_mut().invalid.union(old_bounds);
            self.host.borrow_mut().invalid.union(new_bounds);
        }
    }
    pub fn new_sub(&self) -> Window {
        let w = Rc::new(RefCell::new(WindowData::new(Some(Rc::clone(&self.data)))));
        self.data.borrow_mut().subwindows.push(Rc::clone(&w));
        Window { host: Rc::clone(&self.host), data: w }
    }
    pub fn z_index(&self) -> usize {
        fn index(windows: &Vec<Rc<RefCell<WindowData>>>, window: &Rc<RefCell<WindowData>>) -> usize {
            windows.iter().enumerate().filter(|(_, w)| { Rc::ptr_eq(w, window) }).next().unwrap().0
        }
        if let Some(ref parent) = self.data.borrow().parent {
            index(&parent.borrow().subwindows, &self.data)
        } else {
            index(&self.host.borrow().windows, &self.data)
        }
    }
    pub fn set_z_index(&self, index: usize) {
        fn set_index(windows: &mut Vec<Rc<RefCell<WindowData>>>, window: &Rc<RefCell<WindowData>>, index: usize) {
             let old = windows.iter().enumerate().filter(|(_, w)| { Rc::ptr_eq(w, window) }).next().unwrap().0;
             let window = windows.remove(old);
             let index = min(index, windows.len());
             windows.insert(index, window);
        }
        fn global(window: &WindowData, child_y: isize, child_x: isize) -> Option<(isize, isize)> {
            window.bounds.loc()
                .map(|(y, x)| (y + child_y, x + child_x))
                .and_then(|(y, x)| window.parent.as_ref().map_or(Some((y, x)), |parent| global(&parent.borrow(), y, x)))
        }
        let mut bounds = *&self.data.borrow().bounds;
        if let Some((parent_y, parent_x)) = self.data.borrow().parent.as_ref().map_or(Some((0, 0)), |parent| global(&parent.borrow(), 0, 0)) {
            bounds.offset(parent_y, parent_x);
            self.host.borrow_mut().invalid.union(bounds);
        }
        if let Some(ref parent) = self.data.borrow().parent {
            set_index(&mut parent.borrow_mut().subwindows, &self.data, index)
        } else {
            set_index(&mut self.host.borrow_mut().windows, &self.data, index)
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        fn del_window(windows: &mut Vec<Rc<RefCell<WindowData>>>, window: &Rc<RefCell<WindowData>>) {
            let i = windows.iter().enumerate().filter(|(_, w)| { Rc::ptr_eq(w, window) }).next().unwrap().0;
            windows.remove(i);
        }
        self.set_bounds(Rect::empty());
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
    use window::Rect;
    use window::Window;
    use window::WindowData;
    use window::WindowsHost;
    use scr::tests::TestScr;

    use scr::Color;
    use scr::Attr;
    use scr::Texel;

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
        w.scr(&mut s, 0, 0, 100, 100, &mut invalid);
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

    #[test]
    fn outscreen_window() {
        let mut s = TestScr::new(100, 100);
        let host = WindowsHost::new();
        let window = host.new_window();
        window.set_bounds(Rect::tlhw(-1, -5, 1, 2));
        window.out(0, 0, Texel { ch: '+', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        window.out(0, 1, Texel { ch: '-', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        host.scr(&mut s);
    }

    #[test]
    fn set_window_bounds_invalid_crop() {
        let mut s = TestScr::new(100, 100);
        let host = WindowsHost::new();
        let window = host.new_window();
        window.set_bounds(Rect::tlhw(-10, -20, 30, 40));
        {
            let sub = window.new_sub();
            sub.set_bounds(Rect::tlhw(10, 20, 10, 15));
            host.scr(&mut s);
            assert_eq!(Rect::empty(), sub.data.borrow().invalid);
            sub.out(0, 0, Texel { ch: '+', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
            assert_eq!(Rect::tlhw(0, 0, 1, 1), sub.data.borrow().invalid);
            sub.set_bounds(Rect::tlhw(10, 20, 9, 14));
            assert_eq!(Rect::tlhw(0, 0, 1, 1), sub.data.borrow().invalid);
            assert_eq!(Rect::tlhw(0, 0, 10, 15), host.val.borrow().invalid);
            host.scr(&mut s);
            assert_eq!(Rect::empty(), host.val.borrow().invalid);
        }
        assert_eq!(Rect::tlhw(0, 0, 9, 14), host.val.borrow().invalid);
    }

    #[test]
    fn window_z_index() {
        fn fill3x3(window: &Window, fg: Color) {
            window.out(0, 0, Texel { ch: '1', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(0, 1, Texel { ch: '2', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(0, 2, Texel { ch: '3', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(1, 0, Texel { ch: '4', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(1, 1, Texel { ch: '5', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(1, 2, Texel { ch: '6', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(2, 0, Texel { ch: '7', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(2, 1, Texel { ch: '8', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
            window.out(2, 2, Texel { ch: '9', attr: Attr::NORMAL, fg: fg, bg: Some(Color::Black) });
        }
        let mut scr = TestScr::new(4, 4);
        let host = WindowsHost::new();
        let window1 = host.new_window();
        window1.set_bounds(Rect::tlhw(0, 0, 3, 3));
        fill3x3(&window1, Color::Green);
        let window2 = host.new_window();
        window2.set_bounds(Rect::tlhw(1, 1, 3, 3));
        fill3x3(&window2, Color::Red);
        host.scr(&mut scr);
        assert_eq!([
            Texel { ch: '1', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '2', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '3', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: 'T', attr: Attr::NORMAL, fg: Color::Cyan, bg: Some(Color::Red) },
            Texel { ch: '4', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '1', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '2', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '3', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '7', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '4', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '5', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '6', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: 'T', attr: Attr::NORMAL, fg: Color::Cyan, bg: Some(Color::Red) },
            Texel { ch: '7', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '8', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '9', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
        ], &*scr.content);
        assert_eq!(0, window1.z_index());
        assert_eq!(1, window2.z_index());
        window1.set_z_index(5);
        assert_eq!(0, window2.z_index());
        assert_eq!(1, window1.z_index());
        host.scr(&mut scr);
        assert_eq!([
            Texel { ch: '1', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '2', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '3', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: 'T', attr: Attr::NORMAL, fg: Color::Cyan, bg: Some(Color::Red) },
            Texel { ch: '4', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '5', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '6', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '3', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '7', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '8', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '9', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) },
            Texel { ch: '6', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: 'T', attr: Attr::NORMAL, fg: Color::Cyan, bg: Some(Color::Red) },
            Texel { ch: '7', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '8', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
            Texel { ch: '9', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) },
        ], &*scr.content);
    }

    #[test]
    fn double_scr() {
        let mut scr = TestScr::new(10, 136);
        let host = WindowsHost::new();
        let window = host.new_window();
        window.set_bounds(Rect::tlhw(0, 0, 10, 136));
        window.out(6, 133, Texel { ch: 'A', attr: Attr::NORMAL, fg: Color::Green, bg: None });
        window.out(6, 134, Texel { ch: 'B', attr: Attr::NORMAL, fg: Color::Green, bg: None });
        window.out(6, 135, Texel { ch: 'c', attr: Attr::NORMAL, fg: Color::Green, bg: None });
        window.out(5, 5, Texel { ch: 'l', attr: Attr::ALTCHARSET | Attr::REVERSE, fg: Color::Green, bg: Some(Color::Black) });
        host.scr(&mut scr);
        window.out(6, 2, Texel { ch: 'i', attr: Attr::UNDERLINE, fg: Color::Red, bg: None });
        host.scr(&mut scr);
        assert_eq!(Texel { ch: 'i', attr: Attr::UNDERLINE, fg: Color::Red, bg: None }, scr.content[6 * 136 + 2]);
    }

    #[test]
    fn windows_hierarhy() {
        let mut scr = TestScr::new(4, 4);
        let host = WindowsHost::new();
        let window1 = host.new_window();
        window1.set_bounds(Rect::tlhw(0, 0, 4, 2));
        let window2 = host.new_window();
        window2.set_bounds(Rect::tlhw(0, 2, 4, 2));
        let sub1 = window1.new_sub();
        sub1.set_bounds(Rect::tlhw(1, 0, 3, 2));
        let sub2 = window2.new_sub();
        sub2.set_bounds(Rect::tlhw(0, 0, 3, 2));
        let sub3 = window2.new_sub();
        sub3.set_bounds(Rect::tlhw(0, 1, 3, 2));
        let subsub = sub2.new_sub();
        subsub.set_bounds(Rect::tlhw(1, 1, 1, 1));
        window1.out(0, 0, Texel { ch: 'a', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) });
        window1.out(0, 1, Texel { ch: 'b', attr: Attr::NORMAL, fg: Color::Red, bg: Some(Color::Black) });
        sub2.out(0, 0, Texel { ch: 'D', attr: Attr::NORMAL, fg: Color::Green, bg: Some(Color::Black) });
        host.scr(&mut scr);
    }
}
