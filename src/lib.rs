type Priority = u8;

struct RouterParameters {
    priority: Priority,
}

impl Default for  RouterParameters {
    fn default() -> Self {
        Self {
            priority: 100,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Input {
    Startup,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    WaitForInput,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Initialized,
    Backup,
    Master,
}

pub struct VirtualRouter {
    parameters: RouterParameters,
    state: State,
}

impl VirtualRouter {
    pub fn new(parameters: RouterParameters) -> Self {
        Self {
            state: State::Initialized,
            parameters,
        }
    }

    pub fn handle_input(&mut self, input: Input) -> Action {
        if self.parameters.priority == 255 {
            self.state = State::Master;
        } else {
            self.state = State::Backup;
        }
        Action::WaitForInput
    }

    pub fn state(&self) -> &State {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::{assert_eq};

    #[test]
    fn startup() {
        let mut router = VirtualRouter::new(RouterParameters::default());
        assert_eq!(*router.state(), State::Initialized, "all routers should begin in the initialized state");

        assert_eq!(router.handle_input(Input::Startup), Action::WaitForInput);
        assert_eq!(*router.state(), State::Backup, "after startup, an un-owned router should transition to the Backup state");
    }

    #[test]
    fn startup_address_owner() {
        let mut router = VirtualRouter::new(RouterParameters { priority: 255 /* owner */ });
        assert_eq!(*router.state(), State::Initialized, "all routers should begin in the initialized state");

        assert_eq!(router.handle_input(Input::Startup), Action::WaitForInput);
        assert_eq!(*router.state(), State::Master, "after startup, an owned router should transition to the Master state");
    }
}
