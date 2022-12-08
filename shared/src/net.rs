
pub const PROTOCOL_VERSION: u16 = 0;
pub const PROTOCOL_MAGIC: u16 = 0xB7C1;

pub const MAX_ONLINE_PLAYERS: u16 = 64;

pub type RawNetworkId = u16;

// A per-entity unique identifier shared with all connected clients to identify entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkId(RawNetworkId);

impl NetworkId {
    pub const INVALID : NetworkId = Self::from_raw(0);

    pub const fn from_raw(raw: RawNetworkId) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> RawNetworkId {
        self.0
    }
}

impl std::fmt::Display for NetworkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("NID({})", self.raw()))
    }
}

