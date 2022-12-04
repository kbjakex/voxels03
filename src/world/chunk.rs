use glam::{IVec3, UVec3};

use crate::util;

use super::block::Block;

pub const CHUNK_SIZE_LOG2: usize = 4;
pub const CHUNK_SIZE: usize = 1 << CHUNK_SIZE_LOG2;
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChunkFace {
    NX, NY, NZ, PX, PY, PZ,
}

pub type WorldBlockPos = IVec3;

pub trait WorldBlockPosExt {
    fn to_block_index(self) -> usize;

    fn to_local(self) -> ChunkBlockPos;

    fn to_chunk_pos(self) -> IVec3;
}

impl WorldBlockPosExt for WorldBlockPos {
    fn to_block_index(self) -> usize {
        const XZ_BITS: usize = 24;

        ((self.y as u32 as usize) << (2 * XZ_BITS))
            | ((self.z as u32 as usize) << XZ_BITS)
            | (self.x as u32 as usize)
    }

    fn to_local(self) -> ChunkBlockPos {
        ChunkBlockPos::new(self.x as u8, self.y as u8, self.z as u8)
    }

    fn to_chunk_pos(self) -> IVec3 {
        self >> CHUNK_SIZE_LOG2 as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkBlockPos {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub extra: u8,
}

impl ChunkBlockPos {
    pub const COORD_MASK: u8 = CHUNK_SIZE as u8 - 1;

    pub const fn new(x: u8, y: u8, z: u8) -> Self {
        Self {
            x: x & Self::COORD_MASK,
            y: y & Self::COORD_MASK,
            z: z & Self::COORD_MASK,
            extra: 0,
        }
    }

    pub const fn to_block_index(self) -> usize {
        self.z as usize * CHUNK_SIZE * CHUNK_SIZE + self.x as usize * CHUNK_SIZE + self.y as usize
    }
}

impl From<WorldBlockPos> for ChunkBlockPos {
    fn from(pos: WorldBlockPos) -> Self {
        pos.to_local()
    }
}

pub struct Chunk {
    // Todo: swap to palette compression
    blocks: [Block; CHUNK_VOLUME],
}

impl Chunk {
    pub fn new() -> Box<Self> {
        // This dance is to work around the fact that Rust has no placement new.
        // I don't want to rely on the on-stack construction being optimized away,
        // especially because on debug mode it definitely isn't, and I like being
        // able to debug, and this causes a heightened increased risk of stack overflow.
        let boxed = unsafe { util::boxed_zeroed::<Chunk>() };

        // For some reason, the compiler does not optimize the memset away,
        // even though the memory is already zero-initialized...and air 
        // block is just zero bits.
        const _: () = assert!(
            Block::AIR.raw() == 0,
            "Chunk::new(): air block no longer zero, needs memset"
        );
        // boxed.blocks.fill(Block::new(BlockId::AIR));

        boxed
    }

    #[inline(always)]
    pub fn get_at(&self, pos: impl Into<UVec3>) -> Block {
        self.blocks[block_idx(pos.into())]
    }

    pub fn set_at(&mut self, pos: impl Into<UVec3>, block: Block) {
        self.blocks[block_idx(pos.into())] = block;
        // Left to do:
        // - update the density map
        // - mark as "lightmap needs to be recomputed" (unless light at this location is zero?)
        // - update bitmaps if we have those on the client as well
        // - mark as "needs to be remeshed"
    }

    #[inline(always)]
    pub fn fill(&mut self, block: Block) {
        self.blocks.fill(block);
    }

    #[inline(always)]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Block> + 'a {
        self.blocks.iter().copied()
    }
}

fn block_idx(UVec3 { x, y, z}: UVec3) -> usize {
    let u = (x << 8) | (z << 4) | y;
    unsafe { // safe: on x86_64
        use std::arch::x86_64::_pext_u32;

        let sect_idx = _pext_u32(u, 0b1100_1110_1100);
        let block_idx = _pext_u32(u, 0b0011_0001_0011);
        sect_idx as usize * 128 + block_idx as usize
    }
}

