//! SSD1362 SPI interface

use embedded_hal as hal;
use hal::digital::v2::OutputPin;

use crate::error::Error;

/// A method of communicating with SSD1306
pub trait DisplayInterface {
    /// Interface error type
    type Error;
    /// Send a batch of up to 8 commands to display.
    fn send_commands(&mut self, cmd: &[u8]) -> Result<(), Self::Error>;
    /// Send data to display.
    fn send_data(&mut self, buf: &[u8]) -> Result<(), Self::Error>;

}


// TODO: Add to prelude
/// SPI display interface.
///
/// This combines the SPI peripheral and a data/command pin
pub struct SpiInterface<SPI, CS, DC> {
    spi: SPI,
    cs: CS,
    dc: DC,
}

impl<SPI, CS, DC, CommE, PinE> SpiInterface<SPI, CS, DC>
where
    SPI: hal::blocking::spi::Write<u8, Error = CommE>,
    CS: OutputPin<Error = PinE>,
    DC: OutputPin<Error = PinE>,
{
    /// Create new SPI interface for communciation with SSD1306
    pub fn new(spi: SPI, cs: CS, dc: DC) -> Self {
        Self { spi, cs, dc }
    }

}

impl<SPI, CS, DC, CommE, PinE> DisplayInterface for SpiInterface<SPI, CS, DC>
where
    SPI: hal::blocking::spi::Write<u8, Error = CommE>,
    CS: OutputPin<Error = PinE>,
    DC: OutputPin<Error = PinE>,
{
    type Error = Error<CommE, PinE>;

    fn send_commands(&mut self, cmds: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low().map_err(Error::Pin)?;
        self.cs.set_low().map_err(Error::Pin)?;
        let res = self.spi.write(&cmds).map_err(Error::Comm);
        self.cs.set_high().map_err(Error::Pin)?;
        res
    }

    fn send_data(&mut self, buf: &[u8]) -> Result<(), Self::Error> {
        // 1 = data, 0 = command
        self.dc.set_high().map_err(Error::Pin)?;

        self.cs.set_low().map_err(Error::Pin)?;
        let res = self.spi.write(&buf).map_err(Error::Comm);
        self.cs.set_high().map_err(Error::Pin)?;
        res
    }

    // fn send_bounded_data(
    //     &mut self,
    //     buf: &[u8],
    //     disp_width: usize,
    //     upper_left: (u8, u8),
    //     lower_right: (u8, u8),
    // ) -> Result<(), Self::Error> {
    //     self.dc.set_high().map_err(Error::Pin)?;

    //     // let height = ((lower_right.1 - upper_left.1)) as usize;

    //     // let starting_page = (upper_left.1) as usize;

    //     // let mut page_offset = starting_page * disp_width;

    //     self.cs.set_low().map_err(Error::Pin)?;

    //     // TODO there shouldn't be any display properties here..
    //     // for _ in 0..=height {
    //     //     let start_index = page_offset + upper_left.0 as usize;
    //     //     let end_index = page_offset + lower_right.0 as usize;
    //     //     let sub_buf = &buf[start_index..end_index];

    //     //     page_offset += disp_width;

    //     //     self.spi.write(&sub_buf).map_err(Error::Comm)?;
    //     // }

    //     self.cs.set_high().map_err(Error::Pin)?;
    //     Ok(())
    // }

}