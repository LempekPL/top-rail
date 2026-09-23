use crate::railway::track::{Track, TrackMut, TrackNode, TrackSegment};
use bevy::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

const SAVE_PLACE: &'static str = "./saves/";

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Messages<SaveGame>>();
        app.init_resource::<Messages<LoadGame>>();
        app.add_systems(Update, (save_game_system, load_game_system));
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
struct SavableSegment {
    id: usize,
    p1: Vec2,
    p2: Vec2,
    start_node: usize,
    end_node: usize,
}

#[derive(Serialize, Deserialize)]
enum SavableNode {
    DeadEnd {
        id: usize,
        pos: Vec2,
        tangent: Vec2,
        track: usize,
    },
    Continuation {
        id: usize,
        pos: Vec2,
        normal: Vec2,
        tracks: [usize; 2],
    },
    // Junction {
    //     id: usize,
    //     pos: Vec2,
    //     normal: Vec2,
    //     outgoing_tracks: Vec<usize>,
    //     active_track_index: usize,
    // },
}

impl SavableNode {
    fn id(&self) -> usize {
        match self {
            SavableNode::DeadEnd { id, .. } | SavableNode::Continuation { id, .. } => *id,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SaveFile {
    nodes: Vec<SavableNode>,
    segments: Vec<SavableSegment>,
}

fn save_game_system(mut ev_save: MessageReader<SaveGame>, track: Track) {
    for ev in ev_save.read() {
        let file_path = format!("{}.ron", ev.name);
        let mut node_map = HashMap::new();
        let mut seg_map = HashMap::new();
        for (i, (entity, _)) in track.nodes.iter().enumerate() {
            node_map.insert(entity, i);
        }
        for (i, (entity, _)) in track.segments.iter().enumerate() {
            seg_map.insert(entity, i);
        }
        let mut save_file = SaveFile {
            nodes: Vec::with_capacity(node_map.len()),
            segments: Vec::with_capacity(seg_map.len()),
        };

        for (entity, node) in track.nodes.iter() {
            let id = node_map[&entity];
            match *node {
                TrackNode::DeadEnd {
                    pos,
                    tangent,
                    track: track_ent,
                } => {
                    save_file.nodes.push(SavableNode::DeadEnd {
                        id,
                        pos,
                        tangent,
                        track: seg_map[&track_ent],
                    });
                }
                TrackNode::Continuation {
                    pos,
                    normal,
                    tracks,
                } => {
                    save_file.nodes.push(SavableNode::Continuation {
                        id,
                        pos,
                        normal,
                        tracks: [seg_map[&tracks[0]], seg_map[&tracks[1]]],
                    });
                }
                TrackNode::Junction { .. } => todo!(),
                TrackNode::Crossing { .. } => todo!(),
            }
        }

        for (entity, segment) in track.segments.iter() {
            let id = seg_map[&entity];
            save_file.segments.push(SavableSegment {
                id,
                p1: segment.p1,
                p2: segment.p2,
                start_node: node_map[&segment.start_node],
                end_node: node_map[&segment.end_node],
            });
        }

        if let Ok(ron) = ron::ser::to_string_pretty(&save_file, PrettyConfig::default()) {
            if fs::write(&file_path, ron).is_ok() {
                info!("Game saved to {}!", file_path);
            } else {
                error!("Failed to save: {}", file_path);
            }
        }
    }
}

fn load_game_system(mut commands: Commands, mut ev_load: MessageReader<LoadGame>, track: TrackMut) {
    for ev in ev_load.read() {
        let file_path = format!("{}.ron", ev.name);

        if let Ok(ron_data) = &fs::read_to_string(&file_path)
            && let Ok(save_file) = ron::de::from_str::<SaveFile>(ron_data)
        {
            for (entity, _) in track.nodes.iter() {
                commands.entity(entity).despawn();
            }
            for (entity, _) in track.segments.iter() {
                commands.entity(entity).despawn();
            }

            let mut node_map = HashMap::new();
            let mut seg_map = HashMap::new();

            for saved_node in &save_file.nodes {
                node_map.insert(saved_node.id(), commands.spawn_empty().id());
            }

            for saved_seg in &save_file.segments {
                seg_map.insert(saved_seg.id, commands.spawn_empty().id());
            }

            for saved_node in &save_file.nodes {
                match saved_node {
                    SavableNode::DeadEnd {
                        id,
                        pos,
                        tangent,
                        track,
                    } => {
                        let entity = node_map[id];
                        let track_ent = seg_map[track];
                        commands
                            .entity(entity)
                            .insert(TrackNode::bundle_end(*pos, *tangent, track_ent));
                    }
                    SavableNode::Continuation {
                        id,
                        pos,
                        normal,
                        tracks,
                    } => {
                        let entity = node_map[id];
                        commands.entity(entity).insert(TrackNode::bundle_cont(
                            *pos,
                            *normal,
                            [seg_map[&tracks[0]], seg_map[&tracks[1]]],
                        ));
                    } // SavableNode::Junction { .. } => todo!(),
                }
            }

            for saved_seg in &save_file.segments {
                let entity = seg_map[&saved_seg.id];
                let start_node = node_map[&saved_seg.start_node];
                let end_node = node_map[&saved_seg.end_node];

                commands.entity(entity).insert(TrackSegment::bundle(
                    saved_seg.p1,
                    saved_seg.p2,
                    start_node,
                    end_node,
                ));
            }

            info!("Save loaded successfully from {}!", file_path);
        } else {
            error!("Failed to load file: {}", file_path);
        }
    }
}
