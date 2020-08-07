

use display_interface::{DisplayError, WriteOnlyDataCommand};
use core::{cmp::min, fmt};

pub use crate::chars::{Font6x8, TerminalFont};
use crate::display::Display;

use heapless::consts::U512;

/// Contains the new row that the cursor has wrapped around to
struct CursorWrapEvent(usize);

use indexed_ringbuffer::IndexedRingbuffer;

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
    /// Returns a value indicating if this caused the cursor to reach the end  to the next line or the next screen.
    pub fn advance(&mut self) -> Option<CursorWrapEvent> {
        self.col = min(self.col + 1, self.width);
        if self.col == self.width {
            // self.row = (self.row - 1) % self.height;
            Some(CursorWrapEvent(self.row))
        } else {
            None
        }
    }

    /// Advances the logical cursor to the start of the next line
    /// Returns a value indicating the now active line
    pub fn advance_line(&mut self) -> CursorWrapEvent {
        self.row = self.row + 1;
        self.col = 0;
        CursorWrapEvent(self.row)
    }

    pub fn get_line_box(&self, offset: usize) -> ((u8, u8), (u8, u8)) {
        let (chr_w,chr_h) = self.char_size;

        let x_end = self.width * chr_w / 2;

        let y_start = (self.height - 1) * chr_h - (self.row - offset) * chr_h;
        let y_end = y_start + chr_h;

        ((0u8, y_start as u8), (x_end as u8, y_end as u8))
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
    wrap: bool,
    num_lines: usize
}

impl<DI, F> RenderEngine<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{

    pub fn new(display: Display<DI>, mut font: F, wrap: bool) -> Self {
        let cursor = Cursor::new(font.char_size(), display.dimensions());

        let num_lines = display.dimensions().1 / font.char_size().1;
        Self {
            display,
            font,
            cursor,
            wrap,
            num_lines
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

        Ok(())
    }

    fn render_all<'a>(&mut self, lines: impl Iterator<Item=&'a[u8]>) -> Result<(), DisplayError> {
        self.cursor.set_position(0,0);

        for line in lines {

            let line_length = if line[line.len()-1] == '\n' as u8 {
                line.len() - 1
            } else {
                line.len()
            };

            // lines more than 1 per element
            let extra_lines = if self.wrap {
                line_length / (self.cursor.width + 1)
            } else {
                0
            };

            for _ in 0..extra_lines {
                self.cursor.advance_line();
            }

            let mut line_offset = 0;

            let draw_area = self.cursor.get_line_box(line_offset);
            self.display.set_draw_area(draw_area.0, draw_area.1)?;

            for byte in line {

                if *byte as char == '\n' {
                    break;
                }

                self.write_char(*byte as char)?;


                if let Some(_wrap) = self.cursor.advance() {
                    if self.wrap && (line_length > self.cursor.width) {
                        line_offset += 1;
                        self.cursor.set_position(0, self.cursor.get_position().1);
                        let draw_area = self.cursor.get_line_box(line_offset);
                        self.display.set_draw_area(draw_area.0, draw_area.1)?;
                    } else {
                        // no wrap, go to next line
                        break;
                    }
                }

            }

            self.fill_blank()?;
            self.cursor.advance_line();

            if self.cursor.get_position().1 >= self.num_lines {
                break;
            }
        }
        Ok(())
    }

    fn fill_blank(&mut self) -> Result<(), DisplayError> {
        if self.cursor.get_position().0 == self.cursor.width {
            return Ok(());
        }
        loop {
            self.write_char(' ')?;
            if let Some(_wrap) = self.cursor.advance() {
                break;
            }
        }
        Ok(())
    }

    fn write_char(&mut self, chr: char) -> Result<(), DisplayError> {

        match chr {
            '\t' => self.draw_char(' ')?,
            '\n' =>  {},
            '\r' => {},
            '\0' => {},
            _ => self.draw_char(chr)?
        }

        Ok(())
    }

    fn draw_char(&mut self, chr: char) -> Result<(), DisplayError> {
        let bitmap = self.font.get_char(chr as u8);
        self.display.draw(&bitmap)?;
        Ok(())
    }
}

pub struct TerminalView<DI, F> {
    render: RenderEngine<DI, F>,
    char_buffer: IndexedRingbuffer<U512>,
    scroll_offset: usize,
}

impl<DI, F> TerminalView<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{
    /// Create new TerminalView instance
    pub fn new(display: Display<DI>, font: F) -> Self {
        TerminalView {
            render: RenderEngine::new(display, font, true),
            char_buffer: IndexedRingbuffer::new(),
            scroll_offset: 0
        }
    }

    pub fn init(&mut self) -> Result<(), DisplayError> {
        self.render.init()?;
        Ok(())
    }

    pub fn write_string(&mut self, s: &str) -> Result<(), DisplayError> {

        self.char_buffer.add(s.as_bytes());

        Ok(())
    }

    pub fn render(&mut self) -> Result<(), DisplayError> {
        self.render.render_all(self.char_buffer.reverse_iter(self.scroll_offset))
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }
}

impl<DI, F> fmt::Write for TerminalView<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        self.write_string(s).unwrap(); // todo error .map_err(err)
        Ok(())
    }
}
