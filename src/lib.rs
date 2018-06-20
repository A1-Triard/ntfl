#![deny(warnings)]
extern crate libc;
extern crate either;
#[macro_use]
extern crate bitflags;

use std::cmp::{ min, max };
//use std::collections::LinkedList;
use std::mem::replace;

pub mod scr;
pub mod ncurses;
use scr::{ Attr, Color, Scr, Texel };

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
            self.val = Some(RectValue { top: x, left: y, height: 1, width: 1 });
        }
    }
    pub fn offset(&mut self, dy: isize, dx: isize) {
        if let Some(ref mut val) = self.val {
            val.top += dy;
            val.left += dx;
        }
    }
    pub fn merge(&mut self, r: Rect) {
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

pub struct Window {
    bounds: Rect,
    content: Vec<Vec<Texel>>,
    invalid: Rect,
}

impl Window {
    pub fn new(bounds: Rect) -> Window {
        let (height, width) = bounds.size();
        Window {
            bounds: bounds,
            content: vec![vec![Texel { ch: 'X', attr: Attr::BOLD, fg: Color::Red, bg: None }; width as usize]; height as usize],
            invalid: Rect::empty()
        }
    }
    pub fn set_bounds(&mut self, bounds: Rect) {
        let (height, width) = bounds.size();
        for row in &mut self.content {
            row.resize(width as usize, Texel { ch: 'X', attr: Attr::BOLD, fg: Color::Red, bg: None });
        }
        self.content.resize(height as usize, vec![Texel { ch: 'X', attr: Attr::BOLD, fg: Color::Red, bg: None }; height as usize]);
        self.invalid = self.invalid.intersection(&bounds);
    }
    pub fn out(&mut self, y: isize, x: isize, ch: char, attr: Attr, fg: Color, bg: Option<Color>) {
        self.invalid.include(y, x);
        replace(&mut self.content[y as usize][x as usize], Texel { ch: ch, attr: attr, fg: fg, bg: bg });
    }
    pub fn scr(&mut self, s: &mut Scr, global_invalid: &mut Rect) {
        let mut invalid = replace(&mut self.invalid, Rect::empty());
        if let Some((y, x)) = self.bounds.loc() {
            invalid.offset(y, x);
            global_invalid.merge(invalid);
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


//pub struct WindowsHost {
    //windows: LinkedList<Window>,
//}



//struct Row {
    //texels: Vec<Texel>,
    //invalid: (c_int, c_int),
//}


//impl Window {
    //fn new() -> Window {
        //Window { y: 0, x: 0, height: 0, width: 0, windows: LinkedList::new(), rows: Vec::new() }
    //}
    //fn resize(&mut self, left: c_int, top: c_int, right: c_int, bottom: c_int) {
        //let width = self.width + left + right;
        //let height = self.height + top + bottom;
        //self.rows.resize(height, Vec::with_capacity(width));
        //if left > 0 {
            //for i in 0
        //} else if left < 0 {
        //}
        //self.y = y - top;
        //self.x = x - left;
        //self.height = height + top + bottom;
        //self.width = width + left + right;
    //}
//}

#[cfg(test)]
mod tests {
    use Rect;
    use Window;
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
        let mut w = Window::new(Rect::tlhw(3, 5, 1, 2));
        w.out(0, 0, '+', Attr::NORMAL, Color::Green, Some(Color::Black));
        w.out(0, 1, '-', Attr::NORMAL, Color::Green, Some(Color::Black));
        let mut s = TestScr::new(100, 100);
        let mut invalid = Rect::empty();
        w.scr(&mut s, &mut invalid);
        assert!(Some((3, 5)) == invalid.loc());
        assert!((1, 2) == invalid.size(), format!("({}, {})", invalid.size().0, invalid.size().1));
        assert!('+' == s.content(3, 5).ch, format!("{}", s.content(3, 5).ch));
        assert!('-' == s.content(3, 6).ch, format!("{}", s.content(3, 6).ch));
    }

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
