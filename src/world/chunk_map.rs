use glam::{IVec2, IVec3, Vec3Swizzles};

use super::{chunk::{CHUNK_SIZE, Chunk}};

pub type ChunkIndex = u32;

pub const WORLD_HEIGHT: usize = 256;
pub const WORLD_HEIGHT_CHUNKS: usize = WORLD_HEIGHT / CHUNK_SIZE;

pub struct Chunks {
    chunks: Box<[Option<Box<Chunk>>]>,
    offset: IVec2,
}

impl Chunks {
    pub fn new(player_chunk_xz: IVec2) -> Self {
        let chunks = std::iter::repeat_with(|| None)
            .take(64 * 16 * 64) // for 32 render distance (32 in front + 32 behind = 64)
            .collect();

        Self {
            offset: player_chunk_xz,
            chunks,
        }
    }

    // Returning a &mut Option is definitely most flexible, but this might be a bad API, todo
    pub fn get_at_mut(&mut self, chunk_pos: IVec3) -> &mut Option<Box<Chunk>> {
        &mut self.chunks[self.pos_to_idx(chunk_pos)]
    }

    pub fn get_at(&self, chunk_pos: IVec3) -> Option<&Chunk> {
        self.chunks[self.pos_to_idx(chunk_pos)].as_deref()
    }

    fn pos_to_idx(&self, chunk_pos: IVec3) -> usize {
        let grid_xz = (chunk_pos.xz() + self.offset).as_uvec2() & 63;
        ((grid_xz.x * 64 * 16) | (grid_xz.y * 16) | (chunk_pos.y as u32 & 15)) as usize
    }
}
