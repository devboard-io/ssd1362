

use display_interface::{DisplayError, WriteOnlyDataCommand};
use core::{cmp::min, fmt};

pub use crate::chars::{Font6x8, TerminalFont};
use crate::display::Display;

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

    pub fn get_char_box(&mut self) -> ((u8, u8), (u8, u8)) {

        let (chr_w,chr_h) = self.char_size;

        let w = chr_w/2;
        let x_start = self.col * w;
        let x_end = x_start + w;

        let y_start = self.row * chr_h;
        let y_end = y_start + chr_h;

        ((x_start as u8, y_start as u8), (x_end as u8, y_end as u8))
    }

    // /// Sets the position of the logical cursor arbitrarily.
    // /// The position will be capped at the maximal possible position.
    // pub fn set_position(&mut self, col: u8, row: u8) {
    //     self.col = min(col, self.width - 1);
    //     self.row = min(row, self.height - 1);
    // }

    // /// Gets the position of the logical cursor on screen in (col, row) order
    // pub fn get_position(&self) -> (u8, u8) {
    //     (self.col, self.row)
    // }

    // /// Gets the logical dimensions of the screen in terms of characters, as (width, height)
    // pub fn get_dimensions(&self) -> (u8, u8) {
    //     (self.width, self.height)
    // }
}


pub struct TerminalView<DI, F> {
    display: Display<DI>,
    cursor: Cursor,
    font: F,
    line_buffer: [[u8;32]; 8], //TODO: should depen on view size and font size
    tabsize: u8,
}

impl<DI, F> TerminalView<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{
    /// Create new TerminalMode instance
    pub fn new(display: Display<DI>, mut font: F) -> Self {
        let cursor = Cursor::new(font.char_size(), display.dimensions());
        TerminalView {
            display,
            cursor,
            font,
            line_buffer: [[0u8;32]; 8],
            tabsize: 4u8
        }
    }

    pub fn init(&mut self) -> Result<(), DisplayError> {
        self.display.init()?;
        self.display.on()?;
        self.clear()?;
        self.write_char(0x1A as char)?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), DisplayError> {

        let buffer: [u8; 128] = [0u8; 128];
        for _i in 0..64 {
            self.display.draw(&buffer)?;
        }

        Ok(())
    }

    pub fn new_line(&mut self) -> Result<(), DisplayError> {
        self.cursor.advance_line();
        self.write_char(0x1A as char)
    }

    pub fn write_string(&mut self, s: &str)  -> Result<(), DisplayError>  {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    pub fn write_char(&mut self, chr: char) -> Result<(), DisplayError> {



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
