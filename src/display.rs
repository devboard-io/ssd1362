use crate::command::{Command, VcomhLevel, DisplayMode};
use crate::interface::DisplayInterface;


///! Display rotation
/// Note that 90º and 270º rotations are not supported by
// [`TerminalMode`](../mode/terminal/struct.TerminalMode.html).
#[derive(Clone, Copy)]
pub enum DisplayRotation {
    /// No rotation, normal display
    Rotate0,
    /// Rotate by 90 degress clockwise
    Rotate90,
    /// Rotate by 180 degress clockwise
    Rotate180,
    /// Rotate 270 degress clockwise
    Rotate270,
}

/// Display size enumeration
#[derive(Clone, Copy)]
pub enum DisplaySize {
    /// 256 by 64 pixels
    Display256x64,
}

impl DisplaySize {
    /// Get integral dimensions from DisplaySize
    pub fn dimensions(&self) -> (usize, usize) {
        match *self {
            // TODO should this be 128 or 256 (since columns are two pixels wide)?
            DisplaySize::Display256x64 => (256, 64),
        }
    }
}

pub struct Display<DI> {
    iface: DI,
    rotation: DisplayRotation,
    size: DisplaySize,
}


impl<DI> Display<DI>
where
    DI: DisplayInterface,
{
    pub fn new(iface: DI, rotation: DisplayRotation) -> Display<DI> {
        let size = DisplaySize::Display256x64;

        Display {
            iface,
            rotation,
            size,
            displaybuffer: [0; 256*64] // TODO: buffer size depends on display size
        }
    }

    pub fn init(&mut self) -> Result<(), DI::Error> {

        Command::InternalVDD(true).send(&mut self.iface)?;
        Command::InternalIREF(true).send(&mut self.iface)?;
        Command::ColumnAddress(0, 0x7f).send(&mut self.iface)?;
        Command::RowAddress(0, 0x3f).send(&mut self.iface)?;

        let remap = match self.rotation {
            DisplayRotation::Rotate0 => 0x43,
            DisplayRotation::Rotate180 => 0x50,
            //TODO implement 90 and 270 rotations
            DisplayRotation::Rotate90 => 0x00,
            DisplayRotation::Rotate270 => 0x00
        };
        Command::Remap(remap).send(&mut self.iface)?;

        Command::StartLine(0).send(&mut self.iface)?;
        Command::DisplayOffset(0).send(&mut self.iface)?;
        Command::Mode(DisplayMode::Normal).send(&mut self.iface)?;
        Command::Multiplex(0x3F).send(&mut self.iface)?;
        Command::PhaseLength(0x11).send(&mut self.iface)?;
        Command::DisplayClockDiv(0xF, 0x0).send(&mut self.iface)?;
        Command::DefaultGrayScale().send(&mut self.iface)?;
        Command::PreChargeVoltage(0x04).send(&mut self.iface)?;
        Command::VcomhDeselect(VcomhLevel::V082).send(&mut self.iface)?;

        Ok(())
    }

    pub fn dimensions(&self) -> (usize, usize) {
        let (w, h) = self.size.dimensions();

        match self.rotation {
            DisplayRotation::Rotate0 | DisplayRotation::Rotate180 => (w, h),
            DisplayRotation::Rotate90 | DisplayRotation::Rotate270 => (h, w),
        }
    }

    /// Send the data to the display for drawing at the current position in the framebuffer
    /// and advance the position accordingly. Cf. `set_draw_area` to modify the area affected by
    /// this method in horizontal / vertical mode.
    pub fn draw(&mut self, buffer: &[u8]) -> Result<(), DI::Error> {
        self.iface.send_data(&buffer)
    }

    /// Turn the display on.
    pub fn on(&mut self) -> Result<(), DI::Error> {
        Command::DisplayOn(true).send(&mut self.iface)
    }

    /// Turn the display off.
    pub fn off(&mut self) -> Result<(), DI::Error> {
        Command::DisplayOn(false).send(&mut self.iface)
    }


    pub fn write_char(&mut self, chr: &[u8; 32], c: u8) -> Result<(), DI::Error> {

        // let four: [u8; 32] = [
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        //     0xFF,0xFF,  /* 111111111111 */
        // ];

        let mut bitmap: [u8; 4*32] = [0; 4*32];

        let mut index = 0;
        let mut nibble = 0;
        for i in 0..chr.len() {

            let byte = chr[i];

            for m in 0..8_u8 {
                let r = byte & (1 << (7-m));

                if r != 0 {
                    bitmap[index] |= 0x0F << (4*nibble);
                }

                nibble += 1;

                if nibble > 1 {
                    index += 1;
                    nibble = 0;
                }
            }
        }


        Command::ColumnAddress(c*8, c*8+8 - 1).send(&mut self.iface)?;
        Command::RowAddress(0, 16 - 1).send(&mut self.iface)?;
        self.draw(&bitmap)

    }
}
