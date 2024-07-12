use std::collections::HashSet;

use valence::prelude::*;

#[derive(Component)]
pub struct ReservedChunksLayer(HashSet<ChunkPos>);

impl ReservedChunksLayer {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn is_reserved(&self, pos: ChunkPos) -> bool {
        self.0.contains(&pos)
    }

    pub fn set_reserved(&mut self, pos: ChunkPos, reserved: bool) {
        if reserved {
            self.0.insert(pos);
        } else {
            self.0.remove(&pos);
        }
    }
}
