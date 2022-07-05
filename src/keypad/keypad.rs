use chip8::pal::{self, Delay, Keypad};
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub type Keymap = [[u8; 4]; 4];

type Result<T = ()> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    WritePin,
    ReadPin,
    Delay,
}

impl Into<pal::Error> for Error {
    fn into(self) -> pal::Error {
        pal::Error::Keypad
    }
}

pub struct GpioKeypad<C1, C2, C3, C4, R1, R2, R3, R4>
where
    C1: OutputPin,
    C2: OutputPin,
    C3: OutputPin,
    C4: OutputPin,
    R1: InputPin,
    R2: InputPin,
    R3: InputPin,
    R4: InputPin,
{
    col1: C1,
    col2: C2,
    col3: C3,
    col4: C4,
    row1: R1,
    row2: R2,
    row3: R3,
    row4: R4,
    keymap: Keymap,
}

macro_rules! set {
    (1 = $($pin: expr),+) => {
        $($pin.set_high().map_err(|_| Error::WritePin)?);+
    };

    (0 = $($pin: expr),+) => {
        $($pin.set_low().map_err(|_| Error::WritePin)?);+
    };
}

macro_rules! try_col {
    ($self: ident: $pin: ident, $delay: expr, $col: literal) => {
        set!(1 = $self.$pin);
        $self.wait($delay);

        if let Some(key) = $self.try_rows($col)? {
            return Ok(Some(key));
        }

        set!(0 = $self.$pin);
    };
}

macro_rules! try_cols {
    ($self: ident, $delay: expr => $($col: literal = $pin: ident),+) => {
        $(try_col!($self: $pin, $delay, $col));+
    };
}

impl<C1, C2, C3, C4, R1, R2, R3, R4> GpioKeypad<C1, C2, C3, C4, R1, R2, R3, R4>
where
    C1: OutputPin,
    C2: OutputPin,
    C3: OutputPin,
    C4: OutputPin,
    R1: InputPin,
    R2: InputPin,
    R3: InputPin,
    R4: InputPin,
{
    const KEYMAP: Keymap = [
        [0x1, 0x2, 0x3, 0xF],
        [0x4, 0x5, 0x6, 0xE],
        [0x7, 0x8, 0x9, 0xD],
        [0xA, 0x0, 0xB, 0xC],
    ];

    pub fn new(
        col1: C1,
        col2: C2,
        col3: C3,
        col4: C4,
        row1: R1,
        row2: R2,
        row3: R3,
        row4: R4,
    ) -> Self {
        Self {
            col1,
            col2,
            col3,
            col4,
            row1,
            row2,
            row3,
            row4,
            keymap: Self::KEYMAP,
        }
    }

    pub fn with_keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn init(&mut self) -> Result {
        set!(1 = self.col1, self.col2, self.col3, self.col4);
        Ok(())
    }

    fn wait<D: Delay>(&self, delay: &mut D) {
        delay.delay_us(500);
    }

    fn read(&self) -> Result<(bool, bool, bool, bool)> {
        Ok((
            self.row1.is_high().map_err(|_| Error::ReadPin)?,
            self.row2.is_high().map_err(|_| Error::ReadPin)?,
            self.row3.is_high().map_err(|_| Error::ReadPin)?,
            self.row4.is_high().map_err(|_| Error::ReadPin)?,
        ))
    }

    fn try_rows(&self, col: usize) -> Result<Option<u8>> {
        let key = match self.read()? {
            (true, false, false, false) => Some(0),
            (false, true, false, false) => Some(1),
            (false, false, true, false) => Some(2),
            (false, false, false, true) => Some(3),
            _ => None,
        }
        .map(|row| self.keymap[row][col]);

        Ok(key)
    }
}

impl<C1, C2, C3, C4, R1, R2, R3, R4> Keypad for GpioKeypad<C1, C2, C3, C4, R1, R2, R3, R4>
where
    C1: OutputPin,
    C2: OutputPin,
    C3: OutputPin,
    C4: OutputPin,
    R1: InputPin,
    R2: InputPin,
    R3: InputPin,
    R4: InputPin,
{
    type Error = Error;

    fn key_is_pressed(&self) -> Result<bool> {
        let (row1, row2, row3, row4) = self.read()?;
        Ok(row1 || row2 || row3 || row4)
    }

    fn read_key<D: Delay>(&mut self, delay: &mut D) -> Result<Option<u8>> {
        if !self.key_is_pressed()? {
            return Ok(None);
        }

        set!(0 = self.col1, self.col2, self.col3, self.col4);

        let mut read_key = || -> Result<Option<u8>> {
            try_cols!(self, delay => 0 = col1, 1 = col2, 2 = col3, 3 = col4);
            Ok(None)
        };

        let result = (read_key)();
        set!(1 = self.col1, self.col2, self.col3, self.col4);
        result
    }
}
