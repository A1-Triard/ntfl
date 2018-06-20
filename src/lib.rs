#![deny(warnings)]
extern crate libc;
extern crate either;
#[macro_use]
extern crate bitflags;

use std::cmp::max;
//use std::collections::LinkedList;
use std::mem::replace;
use std::os::raw::{ c_int };

pub mod scr;
pub mod ncurses;
use scr::Attr;
use scr::Color;

pub struct Rect {
    pub y: c_int,
    pub x: c_int,
    pub height: c_int,
    pub width: c_int,
}

impl Rect {
    pub fn right(&self) -> c_int { self.x + self.width }
    pub fn bottom(&self) -> c_int { self.y + self.height }
    pub fn contains(&self, y: c_int, x: c_int) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }
    pub fn include(&mut self, y: c_int, x: c_int) {
        if y <= self.y { self.y = y; } else { self.height = max(self.height, y - self.y); }
        if x <= self.x { self.x = x; } else { self.width = max(self.width, x - self.x); }
    }
}

#[derive(Debug, Copy, Clone)]
struct Texel {
    pub ch: char,
    pub attr: Attr,
    pub fg: Color,
    pub bg: Option<Color>,
}

pub struct Window {
    bounds: Rect,
    content: Vec<Vec<Texel>>,
    invalid: Option<Rect>,
}

impl Window {
    pub fn new(bounds: Rect) -> Window {
        let width = bounds.width;
        let height = bounds.height;
        Window { bounds: bounds, content: vec![vec![Texel { ch: ' ', attr: Attr::NORMAL, fg: Color::Black, bg: None }; width as usize]; height as usize], invalid: None }
    }
    pub fn bounds(&self) -> &Rect { &self.bounds }
    pub fn out(&mut self, y: c_int, x: c_int, ch: char, attr: Attr, fg: Color, bg: Option<Color>) {
        if !self.invalid.as_mut().map(|i| i.include(y, x) ).is_some() {
            self.invalid = Some(Rect { y: y, x: x, height: 1, width: 1 });
        }
        replace(&mut self.content[y as usize][x as usize], Texel { ch: ch, attr: attr, fg: fg, bg: bg });
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

    use scr::Scr;
    use scr::Color;
    use scr::Attr;
    use ncurses::NCurses;
    use either::{ Left, Right };

    #[test]
    fn rect_contains() {
        let r = Rect { y: 5, x: 7, height: 10, width: 70 };
        assert!(r.contains(10, 10));
        assert!(r.contains(5, 10));
        assert!(!r.contains(10, 5));
    }

    #[test]
    fn it_works() {
        let mut scr = NCurses::new().unwrap();
        scr.out(6, 133, 'A', Attr::NORMAL, Color::Green, None).unwrap();
        scr.out(6, 134, 'B', Attr::NORMAL, Color::Green, None).unwrap();
        scr.out(6, 135, 'c', Attr::NORMAL, Color::Green, None).unwrap();
        scr.out(5, 5, 'l', Attr::ALTCHARSET | Attr::REVERSE, Color::Green, Some(Color::Black)).unwrap();
        scr.refresh(Some((5, 5))).unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => { scr.out(6, 2, c, Attr::UNDERLINE, Color::Red, None).unwrap(); }
        }
        scr.refresh(None).unwrap();
        scr.getch().unwrap();
    }
}
