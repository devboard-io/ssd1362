

use display_interface::{DisplayError, WriteOnlyDataCommand};
use core::{cmp::min, fmt};

pub use crate::chars::{Font6x8, TerminalFont};
use crate::display::Display;

use heapless::spsc::Queue;
use heapless::consts::*;

/// Contains the new row that the cursor has wrapped around to
struct CursorWrapEvent(usize);

struct Cursor {
    col: usize,
    row: usize,
    width: usize,
    height: usize,
    char_size: (usize, usize),
}

impl Cursor {
    pub fn new(char_size: (usize, usize), display_dimensions: (usize, usize)) -> Self {
        let (chr_width, chr_height) = char_size;

        let width = display_dimensions.0 / chr_width;
        let height = display_dimensions.1 / chr_height;
        Cursor {
            col: 0,
            row: 0,
            width,
            height,
            char_size
        }
    }

    /// Advances the logical cursor by one character.
    /// Returns a value indicating if this caused the cursor to wrap to the next line or the next screen.
    pub fn advance(&mut self) -> Option<CursorWrapEvent> {
        self.col = (self.col + 1) % self.width;
        if self.col == 0 {
            self.row = (self.row + 1) % self.height;
            Some(CursorWrapEvent(self.row))
        } else {
            None
        }
    }

    /// Advances the logical cursor to the start of the next line
    /// Returns a value indicating the now active line
    pub fn advance_line(&mut self) -> CursorWrapEvent {
        self.row = (self.row + 1) % self.height;
        self.col = 0;
        CursorWrapEvent(self.row)
    }

    pub fn get_char_box(&self) -> ((u8, u8), (u8, u8)) {

        let (chr_w,chr_h) = self.char_size;

        let w = chr_w/2;
        let x_start = self.col * w;
        let x_end = x_start + w;

        let y_start = self.row * chr_h;
        let y_end = y_start + chr_h;

        ((x_start as u8, y_start as u8), (x_end as u8, y_end as u8))
    }

    /// Sets the position of the logical cursor arbitrarily.
    /// The position will be capped at the maximal possible position.
    pub fn set_position(&mut self, col: usize, row: usize) {
        self.col = min(col, self.width - 1);
        self.row = min(row, self.height - 1);
    }

    /// Gets the position of the logical cursor on screen in (col, row) order
    pub fn get_position(&self) -> (usize, usize) {
        (self.col, self.row)
    }

    // /// Gets the logical dimensions of the screen in terms of characters, as (width, height)
    // pub fn get_dimensions(&self) -> (u8, u8) {
    //     (self.width, self.height)
    // }
}

struct RenderEngine<DI, F> {
    display: Display<DI>,
    font:  F,
    cursor: Cursor,
    tabsize: u8,
}

impl<DI, F> RenderEngine<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{

    pub fn new(display: Display<DI>, mut font: F, tabsize: u8) -> Self {
        let cursor = Cursor::new(font.char_size(), display.dimensions());

        Self {
            display,
            font,
            cursor,
            tabsize
        }
    }

    pub fn init(&mut self) -> Result<(), DisplayError> {

        self.display.init()?;
        self.display.on()?;
        self.clear()?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), DisplayError> {

        self.display.set_draw_area((0,0), (128, 64))?;

        let buffer: [u8; 128] = [0u8; 128];
        for _i in 0..64 {
            self.display.draw(&buffer)?;
        }
        self.cursor.set_position(0,0);
        self.write_char(0x1A as char)?;

        Ok(())
    }

    fn render(&mut self, queue: &Queue<u8, U512, u16>) -> Result<(), DisplayError> {

        let (x,y) = self.cursor.get_position();
        self.clear()?;
        if y >= 7 {
        }
        for byte in queue.into_iter() {
            let chr = *byte;
            self.write_char(chr as char)?;
        }
        Ok(())
    }

    pub fn new_line(&mut self) -> Result<(), DisplayError> {
        self.cursor.advance_line();
        self.write_char(0x1A as char)?;
        Ok(())
    }

    fn write_char(&mut self, chr: char) -> Result<(), DisplayError> {

        match chr {
            '\n' => self.new_line()?,
            '\t' => {
                for _ in 0..self.tabsize {
                    self.draw_char(' ')?;
                }
            },
            '\r' => {},
            _ => self.draw_char(chr)?
        }

        Ok(())
    }

    fn draw_char(&mut self, chr: char) -> Result<(), DisplayError> {
        let bitmap = self.font.get_char(chr as u8);
        let draw_area = self.cursor.get_char_box();
        self.display.set_draw_area(draw_area.0, draw_area.1)?;
        self.display.draw(&bitmap)?;
        self.cursor.advance();
        Ok(())
    }
}

pub struct TerminalView<DI, F> {
    render: RenderEngine<DI, F>,
    line_buffer: Queue<u8, U512, u16>,
}

impl<DI, F> TerminalView<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{
    /// Create new TerminalMode instance
    pub fn new(display: Display<DI>, font: F) -> Self {
        TerminalView {
            render: RenderEngine::new(display, font, 4u8),
            line_buffer: Queue::u16(),
        }
    }

    pub fn init(&mut self) -> Result<(), DisplayError> {
        self.render.init()?;
        Ok(())
    }

    pub fn write_string(&mut self, s: &str)  -> Result<(), DisplayError>  {

        let mut free = self.line_buffer.capacity() - self.line_buffer.len();

        // remove lines until enough space
        while free < (s.len() as u16) {

            let mut chr: char = 'A';
            while chr != '\n' {
                match self.line_buffer.dequeue() {
                    Some(c) => chr = c as char,
                    None => break
                }
            }

            free = self.line_buffer.capacity() - self.line_buffer.len();
        }

        for byte in s.as_bytes() {
            self.line_buffer.enqueue(*byte).unwrap(); //todo handle error
        }

        self.render.render(&self.line_buffer)?;

        Ok(())
    }
}
