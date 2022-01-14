//! Traits for emulator IO

/// Random number generator
pub trait Random {
    fn randint(&mut self) -> u8;
}
