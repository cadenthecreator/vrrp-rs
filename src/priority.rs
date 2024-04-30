#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Priority(u8);

impl Priority {
    pub const SHUTDOWN: Priority = Priority(0);
    pub const OWNER: Priority = Priority(255);

    pub const fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    pub const fn new(priority: u8) -> Self {
        Self(priority)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::new(100)
    }
}
