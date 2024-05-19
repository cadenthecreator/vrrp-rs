use crate::ReceivedPacket;

#[derive(Debug, PartialEq)]
pub enum Input {
    Command(Command),
    Packet(ReceivedPacket),
    Timer,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Startup,
    Shutdown,
}

impl From<Command> for Input {
    fn from(command: Command) -> Self {
        Self::Command(command)
    }
}

impl From<ReceivedPacket> for Input {
    fn from(oacket: ReceivedPacket) -> Self {
        Self::Packet(oacket)
    }
}
