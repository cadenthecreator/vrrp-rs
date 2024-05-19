use std::cmp::Ordering;
use std::num::NonZeroU8;

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Priority(NonZeroU8);

impl Priority {
    pub fn as_u16(&self) -> u16 {
        <Priority as Into<NonZeroU8>>::into(*self).get() as u16
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self(NonZeroU8::new(100).unwrap())
    }
}

impl TryFrom<u8> for Priority {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const SHUTDOWN: u8 = 0;
        const OWNER: u8 = 255;

        match value {
            SHUTDOWN | OWNER => Err(()),
            priority => Ok(Self(NonZeroU8::new(priority).unwrap())),
        }
    }
}

impl Into<NonZeroU8> for Priority {
    fn into(self) -> NonZeroU8 {
        self.0
    }
}

impl PartialEq<NonZeroU8> for Priority {
    fn eq(&self, other: &NonZeroU8) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<NonZeroU8> for Priority {
    fn partial_cmp(&self, other: &NonZeroU8) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}
