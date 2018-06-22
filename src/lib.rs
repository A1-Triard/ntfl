#![deny(warnings)]
#[macro_use]
extern crate bitflags;
extern crate either;
extern crate libc;

pub mod scr;
pub mod ncurses;
pub mod window;

use std::char::from_u32;
use scr::{ Color, Attr, Texel };
use window::{ Rect, Window };

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Graph {
    ULCorner = 'l' as u32 as u8,
    URCorner = 'k' as u32 as u8,
    LLCorner = 'm' as u32 as u8,
    LRCorner = 'j' as u32 as u8,
    LTee = 't' as u32 as u8,
    RTee = 'u' as u32 as u8,
    BTee = 'v' as u32 as u8,
    TTee = 'w' as u32 as u8,
    HLine = 'q' as u32 as u8,
    VLine = 'x' as u32 as u8,
    Plus = 'n' as u32 as u8,
    S1 = 'o' as u32 as u8,
    S9 = 's' as u32 as u8,
    Diamond = '`' as u32 as u8,
    CkBoard = 'a' as u32 as u8,
    Degree = 'f' as u32 as u8,
    PlMimus = 'g' as u32 as u8,
    Bullet = '~' as u32 as u8,
    LArrow = ',' as u32 as u8,
    RArrow = '+' as u32 as u8,
    DArrow = '.' as u32 as u8,
    UArrow = '-' as u32 as u8,
    Board = 'h' as u32 as u8,
    Lantern = 'i' as u32 as u8,
    Block = '0' as u32 as u8,
    S3 = 'p' as u32 as u8,
    S7 = 'r' as u32 as u8,
    LessEqual = 'y' as u32 as u8,
    GreaterEqual = 'z' as u32 as u8,
    Pi = '{' as u32 as u8,
    NotEqual = '|' as u32 as u8,
    Sterling = '}' as u32 as u8,
}

pub trait ToTexel {
    fn texel(&self, attr: Attr, fg: Color, bg: Option<Color>) -> Texel;
}

impl ToTexel for Texel {
    fn texel(&self, _: Attr, _: Color, _: Option<Color>) -> Texel {
        *self
    }
}

impl ToTexel for char {
    fn texel(&self, attr: Attr, fg: Color, bg: Option<Color>) -> Texel {
        Texel { ch: *self, attr: attr, fg: fg, bg: bg }
    }
}

impl ToTexel for Graph {
    fn texel(&self, attr: Attr, fg: Color, bg: Option<Color>) -> Texel {
        Texel { ch: from_u32(*self as u8 as u32).unwrap(), attr: attr | Attr::ALTCHARSET, fg: fg, bg: bg }
    }
}

pub fn draw_texel(window: &Window, y: isize, x: isize, t: &ToTexel, attr: Attr, fg: Color, bg: Option<Color>) {
    if window.area().contains(y, x) {
        window.out(y, x, t.texel(attr, fg, bg));
    }
}

pub fn draw_h_line<'a, T: Into<Option<&'a ToTexel>>>(window: &Window, y: isize, x1: isize, x2: isize, attr: Attr, fg: Color, bg: Option<Color>, ch: T) {
    if let Some((x1, x2)) = window.area().inters_h_line(y, x1, x2) {
        let t = ch.into().unwrap_or(&Graph::HLine).texel(attr, fg, bg);
        for x in x1 .. x2 {
            window.out(y, x, t);
        }
    }
}

pub fn draw_v_line<'a, T: Into<Option<&'a ToTexel>>>(window: &Window, y1: isize, y2: isize, x: isize, attr: Attr, fg: Color, bg: Option<Color>, ch: T) {
    if let Some((y1, y2)) = window.area().inters_v_line(y1, y2, x) {
        let t = ch.into().unwrap_or(&Graph::VLine).texel(attr, fg, bg);
        for y in y1 .. y2 {
            window.out(y, x, t);
        }
    }
}

pub struct Border<'a> {
    pub upper_left: Option<&'a ToTexel>,
    pub upper_right: Option<&'a ToTexel>,
    pub lower_left: Option<&'a ToTexel>,
    pub lower_right: Option<&'a ToTexel>,
    pub upper: Option<&'a ToTexel>,
    pub lower: Option<&'a ToTexel>,
    pub left: Option<&'a ToTexel>,
    pub right: Option<&'a ToTexel>,
}

impl<'b> Border<'b> {
    pub fn new() -> Border<'b> {
        Border { upper_left: Some(&Graph::ULCorner), upper_right: Some(&Graph::URCorner), lower_left: Some(&Graph::LLCorner), lower_right: Some(&Graph::LRCorner), upper: Some(&Graph::HLine), lower: Some(&Graph::HLine), left: Some(&Graph::VLine), right: Some(&Graph::VLine) }
    }
    pub fn no_ul(&self) -> Border<'b> {
        Border { upper_left: None, ..*self }
    }
    pub fn ul(&self, t: &'b ToTexel) -> Border<'b> {
        Border { upper_left: Some(t), ..*self }
    }
    pub fn no_ur(&self) -> Border<'b> {
        Border { upper_right: None, ..*self }
    }
    pub fn ur(&self, t: &'b ToTexel) -> Border<'b> {
        Border { upper_right: Some(t), ..*self }
    }
    pub fn no_ll(&self) -> Border<'b> {
        Border { lower_left: None, ..*self }
    }
    pub fn ll(&self, t: &'b ToTexel) -> Border<'b> {
        Border { lower_left: Some(t), ..*self }
    }
    pub fn no_lr(&self) -> Border<'b> {
        Border { lower_right: None, ..*self }
    }
    pub fn lr(&self, t: &'b ToTexel) -> Border<'b> {
        Border { lower_right: Some(t), ..*self }
    }
    pub fn no_upper(&self) -> Border<'b> {
        Border { upper: None, ..*self }
    }
    pub fn upper(&self, t: &'b ToTexel) -> Border<'b> {
        Border { upper: Some(t), ..*self }
    }
    pub fn no_top(&self) -> Border<'b> {
        Border { upper: None, upper_left: None, upper_right: None, ..*self }
    }
    pub fn no_lower(&self) -> Border<'b> {
        Border { lower: None, ..*self }
    }
    pub fn lower(&self, t: &'b ToTexel) -> Border<'b> {
        Border { lower: Some(t), ..*self }
    }
    pub fn no_bottom(&self) -> Border<'b> {
        Border { lower: None, lower_left: None, lower_right: None, ..*self }
    }
    pub fn no_left(&self) -> Border<'b> {
        Border { left: None, ..*self }
    }
    pub fn left(&self, t: &'b ToTexel) -> Border<'b> {
        Border { left: Some(t), ..*self }
    }
    pub fn no_left_side(&self) -> Border<'b> {
        Border { left: None, upper_left: None, lower_left: None, ..*self }
    }
    pub fn no_right(&self) -> Border<'b> {
        Border { right: None, ..*self }
    }
    pub fn right(&self, t: &'b ToTexel) -> Border<'b> {
        Border { right: Some(t), ..*self }
    }
    pub fn no_right_side(&self) -> Border<'b> {
        Border { right: None, upper_right: None, lower_right: None, ..*self }
    }
}

pub fn draw_border(window: &Window, bounds: &Rect, attr: Attr, fg: Color, bg: Option<Color>, border: &Border) {
    if let Some((y, x)) = bounds.loc() {
        let (height, width) = bounds.size();
        if let Some(t) = border.upper { draw_h_line(window, y, x + 1, x + width, attr, fg, bg, t); }
        if let Some(t) = border.lower { draw_h_line(window, y + height, x + 1, x + width, attr, fg, bg, t); }
        if let Some(t) = border.left { draw_v_line(window, y + 1, y + height, x, attr, fg, bg, t); }
        if let Some(t) = border.right { draw_v_line(window, y + 1, y + height, x + width, attr, fg, bg, t); }
        if let Some(t) = border.upper_left { draw_texel(window, y, x, t, attr, fg, bg); }
        if let Some(t) = border.upper_right { draw_texel(window, y, x + width, t, attr, fg, bg); }
        if let Some(t) = border.lower_left { draw_texel(window, y + height, x, t, attr, fg, bg); }
        if let Some(t) = border.lower_right { draw_texel(window, y + height, x + width, t, attr, fg, bg); }
    }
}

#[cfg(test)]
mod tests {
    use either::{ Left, Right };
    use ncurses::NCurses;
    use scr::{ Scr, Color, Attr };
    use window::{ Rect, WindowsHost };
    use { draw_border, draw_texel, Border, Graph };

    #[test]
    fn it_works() {
        let mut scr = NCurses::new().unwrap();
        let host = WindowsHost::new();
        let window = host.new_window();
        let height = scr.get_height().unwrap();
        let width = scr.get_width().unwrap();
        window.set_bounds(Rect::tlhw(0, 0, height, width));
        draw_border(&window, &Rect::tlbr(10, 0, 13, 40), Attr::BOLD, Color::Blue, None, &Border::new().ul(&Graph::LTee).ur(&Graph::RTee));
        draw_border(&window, &Rect::tlbr(0, 0, 10, 40), Attr::BOLD, Color::Blue, None, &Border::new().no_bottom());
        draw_texel(&window, 6, 133, &'A', Attr::NORMAL, Color::Green, None);
        draw_texel(&window, 6, 134, &'B', Attr::NORMAL, Color::Green, None);
        draw_texel(&window, 6, 135, &'c', Attr::NORMAL, Color::Green, None);
        draw_texel(&window, 5, 5, &'t', Attr::ALTCHARSET | Attr::REVERSE, Color::Green, Some(Color::Black));
        host.scr(&mut scr);
        scr.refresh(Some((6, 2))).unwrap();
        match scr.getch().unwrap() {
            Left(_) => { }
            Right(c) => {
                draw_texel(&window, 6, 2, &c, Attr::UNDERLINE, Color::Red, None);
                host.scr(&mut scr);
                scr.refresh(None).unwrap();
                scr.getch().unwrap();
            }
        }
    }
}
