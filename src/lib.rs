#![deny(warnings)]
extern crate libc;
extern crate either;
#[macro_use]
extern crate bitflags;

//use std::collections::LinkedList;
//use std::os::raw::{ c_int };

pub mod scr;

//struct Texel {
    //ch: char,
    //attr: Attr,
    //fg: Color,
    //bg: Option<Color>,
    //view: Option<&Window>,
//}

//struct Row {
    //texels: Vec<Texel>,
    //invalid: (c_int, c_int),
//}

//pub struct Window {
    //y: c_int,
    //x: c_int,
    //height: c_int,
    //width: c_int,
    //windows: LinkedList<Window>,
    //rows: Vec<Row>,
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
    use scr::Scr;
    use scr::Color;
    use scr::Attr;
    use either::{ Left, Right };

    #[test]
    fn it_works() {
        let mut scr = Scr::new().unwrap();
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
