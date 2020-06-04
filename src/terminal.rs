

use display_interface::{DisplayError, WriteOnlyDataCommand};
use core::{cmp::min, fmt};

use crate::chars::{get_char, TerminalChar, Font6x8, TerminalFont};
use crate::display::Display;

/// Contains the new row that the cursor has wrapped around to
struct CursorWrapEvent(usize);

struct Cursor {
    col: usize,
    row: usize,
    width: usize,
    height: usize,
}

impl Cursor {
    pub fn new(width_pixels: usize, height_pixels: usize) -> Self {
        let width = width_pixels / 8;
        let height = height_pixels / 8;
        Cursor {
            col: 0,
            row: 0,
            width,
            height,
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


pub struct TerminalView<DI> {
    display: Display<DI>,
    cursor: Cursor,
}

impl<DI> TerminalView<DI>
where
    DI: WriteOnlyDataCommand,
{
    /// Create new TerminalMode instance
    pub fn new(display: Display<DI>) -> Self {
        let (display_width,display_height) = display.dimensions();
        let cursor = Cursor::new(display_width, display_height);
        TerminalView {
            display,
            cursor,
        }
    }

    pub fn init(&mut self) -> Result<(), DisplayError> {
        self.display.init()?;
        self.display.on()?;
        self.clear()?;
        self.write_char('A', 0,0)?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), DisplayError> {

        let buffer: [u8; 128] = [0u8; 128];
        for _i in 0..64 {
            self.display.draw(&buffer)?;
        }

        Ok(())
    }

    pub fn write_string(&mut self, s: &str, x: u8, y: u8)  -> Result<(), DisplayError>  {
        let mut i: u8 = 0;
        for c in s.chars() {
            self.write_char(c, x+i, y)?;
            i += 1;
        }
        Ok(())
    }

    pub fn write_char(&mut self, chr: char, x: u8, y:u8) -> Result<(), DisplayError> {

        let mut font: Font6x8 = Font6x8 {};
        let (chr_w,chr_h) = font.char_size();
        let bitmap = font.get_char(chr as u8);


        // let chr = get_char(chr as u8);

        // Columns are 2 pixels wide
        let w = chr_w/2;
        let x_start = x * w;
        let x_end = x_start + w;

        let y_start = y * chr_h;
        let y_end = y_start + chr_h;

        self.display.set_draw_area((x_start, y_start), (x_end, y_end))?;
        self.display.draw(&bitmap)
    }
}