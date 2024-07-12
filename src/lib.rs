use serde::{Deserialize, Serialize};
use valence::{math::IVec3, prelude::*};

pub mod reserved_layer;
pub mod structure_layer;

pub struct Structure {
    pub size: IVec3,
    /// position of origin realative to corner
    pub origin_pos: IVec3,
    pub blocks: Vec<BlockState>,
}

impl Structure {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(&SerializableStructure {
            size: self.size.into(),
            origin_pos: self.origin_pos.into(),
            blocks: self.blocks.iter().map(|block| block.to_raw()).collect(),
        })
        .expect("failed to serialize")
    }

    pub fn deserialize(data: &[u8]) -> bincode::Result<Self> {
        let structure = bincode::deserialize::<SerializableStructure>(data)?;
        Ok(Self {
            size: structure.size.into(),
            origin_pos: structure.origin_pos.into(),
            blocks: structure
                .blocks
                .iter()
                .map(|raw| BlockState::from_raw(*raw).unwrap_or(BlockState::AIR))
                .collect(),
        })
    }

    pub fn render_to_layer(&self, layer: &mut ChunkLayer, origin: BlockPos) {
        for (i, block) in self.blocks.iter().enumerate() {
            let i = i as i32;
            let pos = origin - self.origin_pos + Self::index_to_pos(i, self.size);
            layer.set_block(pos, *block);
        }
    }

    pub fn from_section(
        layer: &ChunkLayer,
        corner: BlockPos,
        size: IVec3,
        origin: BlockPos,
    ) -> Self {
        Self {
            size,
            origin_pos: IVec3::new(origin.x, origin.y, origin.z)
                - IVec3::new(corner.x, corner.y, corner.z),
            blocks: (0..size.element_product())
                .map(|i| {
                    layer
                        .block(<[i32; 3]>::from(corner + Self::index_to_pos(i, size)))
                        .map(|b| b.state)
                        .unwrap_or(BlockState::AIR)
                })
                .collect(),
        }
    }

    /// sorry, no bounds check
    pub fn block_at(&self, pos: IVec3) -> BlockState {
        self.blocks[Self::pos_to_index(pos, self.size) as usize]
    }

    pub fn index_to_pos(i: i32, size: IVec3) -> IVec3 {
        IVec3::new(i % size.x, i / size.x % size.y, i / (size.x * size.y))
    }

    pub fn pos_to_index(pos: IVec3, size: IVec3) -> i32 {
        pos.x + pos.y * size.x + pos.z * size.x * size.y
    }
}

#[derive(Serialize, Deserialize)]
struct SerializableStructure {
    size: (i32, i32, i32),
    origin_pos: (i32, i32, i32),
    blocks: Vec<u16>,
}
