
// Block id is a number rather than an enum primarily because
// mapping an int back ot an enum is a nightmare
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BlockId(u16);

impl BlockId {
    const OPAQUE_THRESHOLD : u16 = 512;
    const COLLIDABLE_THRESHOLD : u16 = 256;

    /// Opaque as in *fully* opaque.
    pub fn is_opaque(self) -> bool {
        self.0 >= Self::OPAQUE_THRESHOLD
    }

    pub fn is_collidable(self) -> bool {
        self.0 >= Self::COLLIDABLE_THRESHOLD
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Block(u16);

impl Block {
    pub const fn new(id: BlockId) -> Self {
        Self(id.0)
    }

    /// NOTE: This returns complete bogus if is_complex() is true!
    pub const fn id(self) -> BlockId {
        BlockId(self.0 & 0x3FF) // 10 bits
    }

    /// NOTE: This returns complete bogus if is_complex() is true!
    pub const fn data(self) -> u16 {
        self.0 >> 12 // 4 bits starting from bit 12
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl Block {
    const COMPLEX_FLAG : u16 = 1 << 10;
    const WATERLOGGED_FLAG : u16 = 1 << 11;

    pub const fn is_complex(self) -> bool {
        (self.0 & Self::COMPLEX_FLAG) != 0
    }

    pub const fn is_waterlogged(self) -> bool {
        (self.0 & Self::WATERLOGGED_FLAG) != 0
    }
}


impl BlockId {
    pub const AIR: BlockId = BlockId(0);
    pub const TEST: BlockId = BlockId(1);
}

impl Block {
    pub const AIR: Block = Block::new(BlockId::AIR);
    pub const TEST: Block = Block::new(BlockId::TEST);
}
