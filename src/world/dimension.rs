use super::{chunk_map::Chunks, ecs::ECS};


pub struct Dimension {
    pub chunks: Chunks,
    pub entities: ECS
}
