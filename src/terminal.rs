

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
    /// Returns a value indicating if this caused the cursor to wrap to the next line or the next screen.
    pub fn advance(&mut self) -> Option<CursorWrapEvent> {
        self.col = (self.col + 1) % self.width;
        if self.col == 0 {
            // self.row = (self.row - 1) % self.height;
            Some(CursorWrapEvent(self.row))
        } else {
            None
        }
    }

    /// Advances the logical cursor to the start of the next line
    /// Returns a value indicating the now active line
    pub fn advance_line(&mut self) -> CursorWrapEvent {
        self.row = (self.row + 1); // % self.height;
        self.col = 0;
        CursorWrapEvent(self.row)
    }

    pub fn move_up(&mut self) {
        self.row += 1;
    }

    pub fn move_down(&mut self) {
        self.row -= 1;
    }

    pub fn get_line_box(&self, offset: usize) -> ((u8, u8), (u8, u8)) {
        let (chr_w,chr_h) = self.char_size;

        let x_end = self.width * chr_w / 2;

        let y_start = (self.height - 1) * chr_h - (self.row - offset) * chr_h;
        let y_end = y_start + chr_h;

        ((0u8, y_start as u8), (x_end as u8, y_end as u8))
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
    wrap: bool,
    num_lines: usize
}

impl<DI, F> RenderEngine<DI, F>
where
    DI: WriteOnlyDataCommand,
    F: TerminalFont
{

    pub fn new(display: Display<DI>, mut font: F, tabsize: u8) -> Self {
        let cursor = Cursor::new(font.char_size(), display.dimensions());

        let num_lines = display.dimensions().1 / font.char_size().1;
        Self {
            display,
            font,
            cursor,
            tabsize,
            wrap: true,
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

        // let draw_area = self.cursor.get_line_box(0);
        // self.display.set_draw_area(draw_area.0, draw_area.1)?;

        // self.write_char(0x1A as char)?;

        Ok(())
    }

    fn render_all<'a>(&mut self, lines: impl Iterator<Item=&'a[u8]>) -> Result<(), DisplayError> {
        self.clear()?;

        let num_chars_per_line = self.cursor.width;

        for line in lines {

            let extra_lines = line.len() / num_chars_per_line; // lines more than 1 per element
            for _ in 0..extra_lines {
                self.cursor.advance_line();
            }

            let mut line_offset = 0;

            let draw_area = self.cursor.get_line_box(line_offset);
            self.display.set_draw_area(draw_area.0, draw_area.1)?;

            for byte in line {
                if *byte as char == '\0' {
                    break;
                }

                self.write_char(*byte as char)?;

                if let Some(wrap) = self.cursor.advance() {
                    if self.wrap {
                        line_offset += 1;
                        let draw_area = self.cursor.get_line_box(line_offset);
                        self.display.set_draw_area(draw_area.0, draw_area.1)?;
                    } else {
                        break;
                    }
                }

            }

            if self.cursor.get_position().1 >= self.num_lines {
                break;
            }
        }
        Ok(())
    }

    fn write_char(&mut self, chr: char) -> Result<(), DisplayError> {

        match chr {
            '\n' =>  {
                self.cursor.advance_line();
            },
            '\t' => {
                for _ in 0..self.tabsize {
                    self.draw_char(' ')?;
                }
            },
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
            render: RenderEngine::new(display, font, 4u8),
            char_buffer: IndexedRingbuffer::new(),
            scroll_offset: 0
        }
    }

    pub fn init(&mut self) -> Result<(), DisplayError> {
        self.render.init()?;
        Ok(())
    }

    pub fn write_string(&mut self, s: &str)  -> Result<(), DisplayError> {

        let mut free = self.char_buffer.free();

        // remove lines until enough space
        while free < (s.len() as usize) {
            self.char_buffer.pop();
            free = self.char_buffer.free();
        }

        self.char_buffer.add(s.as_bytes());


        Ok(())
    }

    pub fn render(&mut self) {
        self.render.render_all(self.char_buffer.reverse_iter(self.scroll_offset));
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
