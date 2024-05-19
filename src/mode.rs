use crate::Priority;
use std::net::Ipv4Addr;
use std::num::NonZeroU8;

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Owner,
    Backup(BackupMode),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackupMode {
    pub primary_ip: Ipv4Addr,
    pub priority: Priority,
    pub preempt: bool,
    pub accept: bool,
}

impl From<BackupMode> for Mode {
    fn from(value: BackupMode) -> Self {
        Mode::Backup(value)
    }
}

impl BackupMode {
    pub fn with_primary_ip(primary_ip: Ipv4Addr) -> Self {
        Self {
            primary_ip,
            priority: Priority::default(),
            preempt: true,
            accept: false,
        }
    }

    pub fn with_priority(self, priority: Priority) -> Self {
        Self { priority, ..self }
    }

    pub fn with_preempt(self, preempt: bool) -> Self {
        Self { preempt, ..self }
    }

    pub fn with_accept(self, accept: bool) -> Self {
        Self { accept, ..self }
    }
}

impl Mode {
    pub(crate) fn priority(&self) -> NonZeroU8 {
        match self {
            Mode::Owner => NonZeroU8::MAX,
            Mode::Backup(BackupMode { priority, .. }) => (*priority).into(),
        }
    }

    pub(crate) fn should_accept(&self) -> bool {
        match self {
            Mode::Owner => true,
            Mode::Backup(BackupMode { accept, .. }) => *accept,
        }
    }

    pub(crate) fn should_preempt(&self) -> bool {
        match self {
            Mode::Owner => true,
            Mode::Backup(BackupMode { preempt, .. }) => *preempt,
        }
    }
}
