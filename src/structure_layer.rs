use std::{
    cmp::{max, min},
    collections::HashMap,
    mem,
    sync::Arc,
    thread,
};

use flume::{Receiver, Sender};
use valence::{
    math::{IVec2, IVec3, Vec2Swizzles, Vec3Swizzles},
    prelude::*,
};

use crate::{reserved_layer::ReservedChunksLayer, Structure};

pub struct StructurePlugin;

impl Plugin for StructurePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (strcture_init_system, structrue_update_system));
    }
}

#[derive(Component)]
pub struct StructureLayer {
    structure_map: HashMap<ChunkPos, Vec<Entity>>,
    needs_update: Vec<(ChunkPos, Entity)>,
    sender: Sender<(Arc<Structure>, ChunkPos, UnloadedChunk, BlockPos)>,
    reciever: Receiver<(ChunkPos, UnloadedChunk)>,
}

impl StructureLayer {
    pub fn new() -> Self {
        let (layer_sender, worker_reciever) = flume::unbounded();
        let (worker_sender, layer_reciever) = flume::unbounded();
        let state = Arc::new(ChunkWorkerState {
            sender: worker_sender,
            reciever: worker_reciever,
        });
        let chunk_worker = |s: Arc<ChunkWorkerState>| {
            while let Ok((structure, chunk_pos, mut chunk, structure_pos)) = s.reciever.recv() {
                StructureInstance::place_on_chunk(structure_pos, &structure, chunk_pos, &mut chunk);
                s.sender
                    .send((chunk_pos, chunk))
                    .expect("error woile sending chunk");
            }
        };
        for _ in 0..thread::available_parallelism().unwrap().get() {
            let state = state.clone();
            thread::spawn(move || chunk_worker(state));
        }
        Self {
            structure_map: HashMap::new(),
            needs_update: Vec::new(),
            sender: layer_sender,
            reciever: layer_reciever,
        }
    }
}

#[derive(Component)]
pub struct StructureSource {
    pub source: Arc<Structure>,
}

#[derive(Component)]
pub struct StructureInstance {
    source: Entity,
    position: BlockPos,
    init_state: StructureInstanceInitState,
    layer: Entity,
}

impl StructureInstance {
    pub fn new(source: Entity, layer: Entity, position: BlockPos, keep_loaded: bool) -> Self {
        Self {
            source,
            layer,
            position,
            init_state: if keep_loaded {
                StructureInstanceInitState::KeepLoaded
            } else {
                StructureInstanceInitState::Unload
            },
        }
    }

    pub fn place_on_chunk(
        structure_pos: BlockPos,
        source: &Structure,
        chunk_pos: ChunkPos,
        chunk: &mut UnloadedChunk,
    ) {
        let chunk_pos = IVec3::new(chunk_pos.x * 16, 0, chunk_pos.z * 16);
        let corner = structure_pos - source.origin_pos;
        let local_corner = corner - chunk_pos;
        let (intersect_corner, intersect_size) = intersection(
            IVec2::new(local_corner.x, local_corner.z),
            source.size.xy(),
            IVec2::new(0, 0),
            IVec2::new(16, 16),
        );
        // maybe slightly hacky
        let opposite_corner =
            intersect_corner.xxy().with_y(corner.y) + intersect_size.xxy().with_y(source.size.y);
        for x in intersect_corner.x..opposite_corner.x {
            for y in intersect_corner.y..opposite_corner.y {
                for z in intersect_corner.x..opposite_corner.z {
                    let structrue_local =
                        IVec3::new(x, y, z) + chunk_pos - IVec3::new(corner.x, corner.y, corner.z);
                    chunk.set_block_state(
                        x as u32,
                        y as u32,
                        z as u32,
                        source.block_at(structrue_local),
                    );
                }
            }
        }
    }

    fn chunks(pos: BlockPos, source: &Structure) -> impl Iterator<Item = ChunkPos> {
        let corner_a = pos - source.origin_pos;
        let corner_b = corner_a + source.size;
        let chunk_a = ChunkPos::new(round_down(corner_a.x), round_down(corner_a.z));
        let chunk_b = ChunkPos::new(round_down(corner_b.x), round_down(corner_b.z));
        let chunk_min = min(chunk_a, chunk_b);
        let chunk_max = max(chunk_a, chunk_b);
        (chunk_min.x..=chunk_max.x)
            .map(move |x| (chunk_min.z..chunk_max.z).map(move |z| ChunkPos::new(x, z)))
            .flatten()
    }
}

struct ChunkWorkerState {
    sender: Sender<(ChunkPos, UnloadedChunk)>,
    reciever: Receiver<(Arc<Structure>, ChunkPos, UnloadedChunk, BlockPos)>,
}

fn round_down(v: i32) -> i32 {
    v / 16 - (if v < 0 && v % 16 != 0 { 1 } else { 0 })
}

#[inline]
fn intersection(a_corner: IVec2, a_size: IVec2, b_corner: IVec2, b_size: IVec2) -> (IVec2, IVec2) {
    let a_far_corner = a_corner + a_size;
    let b_far_corner = b_corner + b_size;
    let res_corner = a_corner.max(b_corner);
    let res_far_corner = a_far_corner.min(b_far_corner);
    (res_corner, res_far_corner - res_corner)
}

#[derive(PartialEq, Eq)]
enum StructureInstanceInitState {
    KeepLoaded,
    Unload,
    Initialized,
}

fn strcture_init_system(
    structures: Query<&StructureSource>,
    mut structure_instances: Query<(Entity, &mut StructureInstance), Changed<StructureInstance>>,
    mut layers: Query<(&mut StructureLayer, &mut ChunkLayer, &ReservedChunksLayer)>,
) {
    for (entity, mut structure) in structure_instances.iter_mut() {
        let keep_loaded = match structure.init_state {
            StructureInstanceInitState::KeepLoaded => true,
            StructureInstanceInitState::Unload => false,
            StructureInstanceInitState::Initialized => continue,
        };
        let (mut structure_layer, mut chunk_layer, reserved_layer) = layers
            .get_mut(structure.layer)
            .expect("missing layer components on layer");
        let source = &structures
            .get(structure.source)
            .expect("no structure on source")
            .source;
        if keep_loaded {
            source.render_to_layer(&mut chunk_layer, structure.position);
        }
        for chunk in StructureInstance::chunks(structure.position, source) {
            if keep_loaded || !reserved_layer.is_reserved(chunk) {
                if structure_layer.structure_map.contains_key(&chunk) {
                    structure_layer.structure_map.insert(chunk, Vec::new());
                }
                structure_layer
                    .structure_map
                    .get_mut(&chunk)
                    .unwrap()
                    .push(entity);
                continue;
            }
            structure_layer.needs_update.push((chunk, entity));
        }
        structure.init_state = StructureInstanceInitState::Initialized;
    }
}

fn structrue_update_system(
    mut layers: Query<(
        &mut StructureLayer,
        &mut ChunkLayer,
        &mut ReservedChunksLayer,
    )>,
    structure_instances: Query<&StructureInstance>,
    structure_sources: Query<&StructureSource>,
) {
    for (mut structure_layer, mut chunk_layer, reserved_layer) in layers.iter_mut() {
        for (pos, chunk) in structure_layer.reciever.drain() {
            if !reserved_layer.is_reserved(pos) {
                continue;
            }
            if chunk_layer.insert_chunk(pos, chunk).is_some() {
                eprintln!("inserted chunk that already exists");
            }
        }
        for (chunk_pos, entity) in mem::take(&mut structure_layer.needs_update) {
            if !reserved_layer.is_reserved(chunk_pos) {
                continue;
            }
            let chunk = match chunk_layer.remove_chunk(chunk_pos) {
                Some(c) => c,
                None => {
                    // if the chunk is missing, but rserved readd to list, to process next tick
                    structure_layer.needs_update.push((chunk_pos, entity));
                    continue;
                }
            };
            let instance = match structure_instances.get(entity) {
                Ok(i) => i,
                Err(_) => continue,
            };
            let source = structure_sources
                .get(instance.source)
                .expect("structure source unavalible");
            structure_layer
                .sender
                .send((source.source.clone(), chunk_pos, chunk, instance.position))
                .expect("error while sending");
        }
    }
}
