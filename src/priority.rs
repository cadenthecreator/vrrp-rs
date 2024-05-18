#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Priority(u8);

impl Priority {
    pub const SHUTDOWN: Priority = Priority(0);
    pub const OWNER: Priority = Priority(255);

    pub const fn new(priority: u8) -> Self {
        Self(priority)
    }

    pub const fn as_u16(&self) -> u16 {
        self.0 as u16
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::new(100)
    }
}
