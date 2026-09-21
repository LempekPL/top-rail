// use crate::railway::track::{TrackConnection, TrackNode, TrackSegment};
use crate::state_manager::DespawnWhenMainMenu;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

const SAVE_PLACE: &'static str = "./saves/";

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Messages<SaveGame>>();
        app.init_resource::<Messages<LoadGame>>();
        // app.add_systems(Update, (save_game_system, load_game_system));
    }
}

#[derive(Message)]
pub struct SaveGame {
    pub name: String,
}

#[derive(Message)]
pub struct LoadGame {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
enum SavedConnection {
    Node(usize),
    LoneNode(usize),
    Segment(usize),
    None,
}

#[derive(Serialize, Deserialize)]
struct SavedSegment {
    id: usize,
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    start_node: SavedConnection,
    end_node: SavedConnection,
}

#[derive(Serialize, Deserialize)]
struct SavedNode {
    id: usize,
    position: Vec2,
    outward_tangent: Vec2,
}

#[derive(Serialize, Deserialize)]
struct SaveFile {
    nodes: Vec<SavedNode>,
    segments: Vec<SavedSegment>,
}

// fn map_connection(
//     conn: &TrackConnection,
//     node_map: &HashMap<Entity, usize>,
//     seg_map: &HashMap<Entity, usize>,
// ) -> SavedConnection {
//     match conn {
//         TrackConnection::Node(e) => node_map
//             .get(e)
//             .map(|id| SavedConnection::Node(*id))
//             .unwrap_or(SavedConnection::None),
//         TrackConnection::LoneNode(e) => node_map
//             .get(e)
//             .map(|id| SavedConnection::LoneNode(*id))
//             .unwrap_or(SavedConnection::None),
//         TrackConnection::Segment(e) => seg_map
//             .get(e)
//             .map(|id| SavedConnection::Segment(*id))
//             .unwrap_or(SavedConnection::None),
//         TrackConnection::None => SavedConnection::None,
//     }
// }

// fn unmap_connection(
//     conn: &SavedConnection,
//     node_map: &HashMap<usize, Entity>,
//     seg_map: &HashMap<usize, Entity>,
// ) -> TrackConnection {
//     match conn {
//         SavedConnection::Node(id) => node_map
//             .get(id)
//             .map(|e| TrackConnection::Node(*e))
//             .unwrap_or(TrackConnection::None),
//         SavedConnection::LoneNode(id) => node_map
//             .get(id)
//             .map(|e| TrackConnection::LoneNode(*e))
//             .unwrap_or(TrackConnection::None),
//         SavedConnection::Segment(id) => seg_map
//             .get(id)
//             .map(|e| TrackConnection::Segment(*e))
//             .unwrap_or(TrackConnection::None),
//         SavedConnection::None => TrackConnection::None,
//     }
// }

// pub fn save_game_system(
//     mut ev_save: MessageReader<SaveGame>,
//     q_nodes: Query<(Entity, &Transform, &TrackNode)>,
//     q_segments: Query<(Entity, &TrackSegment)>,
// ) {
//     for ev in ev_save.read() {
//         let file_path = format!("{}.json", ev.name);
//         let mut node_map = HashMap::new();
//         let mut seg_map = HashMap::new();
//         let mut save_file = SaveFile {
//             nodes: Vec::new(),
//             segments: Vec::new(),
//         };
//
//         for (i, (entity, transform, node)) in q_nodes.iter().enumerate() {
//             node_map.insert(entity, i);
//             save_file.nodes.push(SavedNode {
//                 id: i,
//                 position: transform.translation.truncate(),
//                 outward_tangent: node.outward_tangent,
//             });
//         }
//
//         for (i, (entity, segment)) in q_segments.iter().enumerate() {
//             seg_map.insert(entity, i);
//             save_file.segments.push(SavedSegment {
//                 id: i,
//                 p0: segment.p0,
//                 p1: segment.p1,
//                 p2: segment.p2,
//                 p3: segment.p3,
//                 start_node: map_connection(&segment.start_node, &node_map, &seg_map),
//                 end_node: map_connection(&segment.end_node, &node_map, &seg_map),
//             });
//         }
//
//         if let Ok(json) = serde_json::to_string_pretty(&save_file) {
//             if fs::write(&file_path, json).is_ok() {
//                 info!("Game saved to {}!", file_path);
//             } else {
//                 error!("Failed to save: {}", file_path);
//             }
//         }
//     }
// }
//
// pub fn load_game_system(
//     mut commands: Commands,
//     mut ev_load: MessageReader<LoadGame>,
//     q_nodes: Query<(Entity, &Transform, &TrackNode)>,
//     q_segments: Query<(Entity, &TrackSegment)>,
// ) {
//     for ev in ev_load.read() {
//         let file_path = format!("{}.json", ev.name);
//
//         if let Ok(json) = fs::read_to_string(&file_path) {
//             if let Ok(save_file) = serde_json::from_str::<SaveFile>(&json) {
//                 // clear current tracks
//                 for (entity, _, _) in q_nodes.iter() { commands.entity(entity).despawn(); }
//                 for (entity, _) in q_segments.iter() { commands.entity(entity).despawn(); }
//
//                 let mut node_map = HashMap::new();
//                 let mut seg_map = HashMap::new();
//
//                 for saved_node in &save_file.nodes {
//                     let entity = commands.spawn_empty().id();
//                     node_map.insert(saved_node.id, entity);
//                 }
//                 for saved_seg in &save_file.segments {
//                     let entity = commands.spawn_empty().id();
//                     seg_map.insert(saved_seg.id, entity);
//                 }
//
//                 for saved_node in &save_file.nodes {
//                     let entity = node_map[&saved_node.id];
//                     commands.entity(entity).insert(TrackNode::new_transform(
//                         saved_node.position,
//                         saved_node.position - saved_node.outward_tangent,
//                     ));
//                 }
//
//                 for saved_seg in &save_file.segments {
//                     let entity = seg_map[&saved_seg.id];
//                     commands.entity(entity).insert((
//                         TrackSegment {
//                             p0: saved_seg.p0,
//                             p1: saved_seg.p1,
//                             p2: saved_seg.p2,
//                             p3: saved_seg.p3,
//                             start_node: unmap_connection(&saved_seg.start_node, &node_map, &seg_map),
//                             end_node: unmap_connection(&saved_seg.end_node, &node_map, &seg_map),
//                         },
//                         DespawnWhenMainMenu,
//                     ));
//                 }
//
//                 info!("Save loaded successfully from {}!", file_path);
//             } else {
//                 error!("Failed to parse JSON from: {}", file_path);
//             }
//         } else {
//             error!("Save file not found or cannot be read: {}", file_path);
//         }
//     }
// }
