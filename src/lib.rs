use serde::{Deserialize, Serialize};
use valence::{math::IVec3, prelude::*};

pub struct Structure {
    pub size: IVec3,
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
            let pos = origin - self.origin_pos + block_index_pos(i, self.size);
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
                        .block(<[i32; 3]>::from(corner + block_index_pos(i, size)))
                        .map(|b| b.state)
                        .unwrap_or(BlockState::AIR)
                })
                .collect(),
        }
    }
}

fn block_index_pos(i: i32, size: IVec3) -> IVec3 {
    IVec3::new(i % size.x, i / size.x % size.y, i / (size.x * size.y))
}

#[derive(Serialize, Deserialize)]
struct SerializableStructure {
    size: (i32, i32, i32),
    origin_pos: (i32, i32, i32),
    blocks: Vec<u16>,
}
