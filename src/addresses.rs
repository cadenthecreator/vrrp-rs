use std::net::Ipv4Addr;

#[derive(Clone, Debug, PartialEq)]
pub struct VirtualAddresses(Vec<Ipv4Addr>);

impl TryFrom<Vec<Ipv4Addr>> for VirtualAddresses {
    type Error = ();

    fn try_from(value: Vec<Ipv4Addr>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(());
        }
        Ok(Self(value))
    }
}

impl VirtualAddresses {
    pub fn get(&self, index: u8) -> Option<Ipv4Addr> {
        self.0.get(index as usize).copied()
    }

    pub fn first(&self) -> Ipv4Addr {
        *self.0.first().unwrap()
    }

    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        self.0.contains(&ip)
    }
}
