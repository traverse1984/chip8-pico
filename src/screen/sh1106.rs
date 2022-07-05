use chip8::pal::{self, Screen};
use embedded_hal::{
    blocking::spi::{Write, WriteIter},
    digital::v2::OutputPin,
};

type Result<T = ()> = core::result::Result<T, Error>;

#[derive(Copy, Clone, Debug)]
pub enum Error {
    Spi,
    ChipSelect,
    Mode,
    Reset,
}

impl Into<pal::Error> for Error {
    fn into(self) -> pal::Error {
        pal::Error::Screen
    }
}

/// Incomplete instruction-set implementation for the SH1106 OLED driver, which
/// is the one used by https://www.waveshare.com/wiki/Pico-OLED-1.3.
#[derive(Debug, Copy, Clone)]
pub struct Sh1106<SPI, CS, MD, RS>
where
    SPI: Write<u8> + WriteIter<u8>,
    CS: OutputPin,
    MD: OutputPin,
    RS: OutputPin,
{
    spi: SPI,
    cs: CS,
    mode: MD,
    reset: RS,
    buf: [[u8; 8]; 32],
}

impl<SPI, CS, MD, RS> Sh1106<SPI, CS, MD, RS>
where
    SPI: Write<u8> + WriteIter<u8>,
    CS: OutputPin,
    MD: OutputPin,
    RS: OutputPin,
{
    pub fn new(spi: SPI, chip_select_pin: CS, mode_pin: MD, reset_pin: RS) -> Self {
        Self {
            spi,
            cs: chip_select_pin,
            mode: mode_pin,
            reset: reset_pin,
            buf: [[0; 8]; 32],
        }
    }

    #[inline]
    fn chip_select(&mut self) -> Result {
        self.cs.set_low().map_err(|_| Error::ChipSelect)
    }

    #[inline]
    fn chip_deselect(&mut self) -> Result {
        self.cs.set_high().map_err(|_| Error::ChipSelect)
    }

    #[inline]
    fn set_mode_cmd(&mut self) -> Result {
        self.mode.set_low().map_err(|_| Error::Mode)
    }

    #[inline]
    fn set_mode_data(&mut self) -> Result {
        self.mode.set_high().map_err(|_| Error::Mode)
    }

    #[inline]
    fn write(&mut self, data: &[u8]) -> Result {
        self.chip_select()?;
        self.spi.write(data).map_err(|_| Error::Spi)?;
        self.chip_deselect()
    }

    #[inline]
    fn cmd(&mut self, cmd: u8) -> Result {
        self.set_mode_cmd()?;
        self.write(&[cmd])
    }

    #[inline]
    fn data(&mut self, data: &[u8]) -> Result {
        self.set_mode_data()?;
        self.write(data)
    }

    #[inline]
    fn multibyte_cmd(&mut self, cmd: u8, data: u8) -> Result {
        self.set_mode_cmd()?;
        self.write(&[cmd, data])
    }

    pub fn set_display_start(&mut self, start: u8) -> Result {
        self.multibyte_cmd(0xDC, start)
    }

    pub fn set_vertical_addressing(&mut self) -> Result {
        self.cmd(0x21)
    }

    pub fn set_contrast(&mut self, contrast: u8) -> Result {
        self.multibyte_cmd(0x81, contrast)
    }

    pub fn display_on(&mut self) -> Result {
        self.cmd(0xAF)
    }

    pub fn set_display_offset(&mut self, offset: u8) -> Result {
        self.multibyte_cmd(0xD3, offset)
    }

    pub fn set_dclk_osc_freq(&mut self, setting: u8) -> Result {
        self.multibyte_cmd(0xD5, setting)
    }

    pub fn set_pre_charge_period(&mut self, setting: u8) -> Result {
        self.multibyte_cmd(0xD9, setting)
    }

    pub fn set_vcom_deselect_level(&mut self, level: u8) -> Result {
        self.multibyte_cmd(0xD8, level)
    }

    pub fn set_lower_col_addr(&mut self, col: u8) -> Result {
        self.cmd(col & 0x0F)
    }

    pub fn set_higher_col_addr(&mut self, col: u8) -> Result {
        self.cmd(0x10 | col & 0x7)
    }

    pub fn set_col(&mut self, col: u8) -> Result {
        self.set_lower_col_addr(col)?;
        self.set_higher_col_addr(col >> 4)
    }

    pub fn init(&mut self) -> Result {
        self.reset
            .set_high()
            .and_then(|_| self.reset.set_low())
            .and_then(|_| self.reset.set_high())
            .map_err(|_| Error::Reset)?;

        self.set_display_start(0)?;
        self.set_contrast(0x80)?;
        self.set_vertical_addressing()?;
        self.set_dclk_osc_freq(0x41)?;
        self.set_pre_charge_period(0x22)?;
        self.set_vcom_deselect_level(0x35)?;
        self.set_display_offset(0x60)?;
        self.clear()?;
        self.display_on()
    }

    fn scale(byte: u8) -> [u8; 2] {
        const SCALED_NIBBLE: [u8; 16] = [
            0x00, 0x03, 0x0C, 0x0F, 0x30, 0x33, 0x3C, 0x3F, 0xC0, 0xC3, 0xCC, 0xCF, 0xF0, 0xF3,
            0xFC, 0xFF,
        ];

        let byte = byte.reverse_bits();
        let msb = ((byte >> 4) & 0x0F) as usize;
        let lsb = (byte & 0x0F) as usize;

        [SCALED_NIBBLE[lsb], SCALED_NIBBLE[msb]]
    }
}

impl<SPI, CS, MD, RS> Screen for Sh1106<SPI, CS, MD, RS>
where
    SPI: Write<u8> + WriteIter<u8>,
    CS: OutputPin,
    MD: OutputPin,
    RS: OutputPin,
{
    type Error = Error;

    fn xor(&mut self, x: u8, y: u8, data: &[u8]) -> Result<bool> {
        let offset = x % 8;

        for (scan, ypos) in data.iter().copied().zip(y..) {
            let yidx = ypos as usize;

            // Screen orientation: Highest index is top of screen
            let ypos = 2 * (31 - (ypos % 32));
            let xidx = ((x % 64) / 8) as usize;

            if offset == 0 {
                self.buf[yidx][xidx] ^= scan;
            } else {
                self.buf[yidx][xidx] ^= scan >> offset;
                self.buf[yidx][(xidx + 1) % 8] ^= scan << (8 - offset);
            }

            let draw = unsafe {
                let scaled = self.buf[yidx].map(Self::scale);
                core::mem::transmute::<[[u8; 2]; 8], [u8; 16]>(scaled)
            };

            self.set_col(ypos)?;
            self.data(&draw)?;
            self.set_col(ypos + 1)?;
            self.data(&draw)?;
        }

        // @TODO: Determine if bits have been erased
        Ok(true)
    }

    fn clear(&mut self) -> Result {
        for col in 0..64 {
            self.set_col(col)?;
            self.data(&[0; 16])?;
        }

        self.buf = [[0; 8]; 32];

        Ok(())
    }
}
