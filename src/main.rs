use std::collections::HashSet;

use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use bevy_rapier2d::prelude::*;
use rand::Rng;

fn main() {
    App::build()
        .init_resource::<Game>()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_game.system())
        // .add_system(tetromino_movement.system())
        .add_system(tetromino_sleep_detection.system())
        // .add_system(update_health_bar.system())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .run();
}

const BLOCK_PX_SIZE: f32 = 30.0;

// In terms of block size:
const FLOOR_BLOCK_HEIGHT: f32 = 2.0;
const HEALTH_BAR_HEIGHT: f32 = 0.5;

#[derive(Default)]
struct Stats {
    generated_blocks: i32,
    cleared_blocks: i32,
    lost_blocks: i32,
    lost_tetromino: bool,
}

impl Stats {
    fn health(&self) -> f32 {
        if self.lost_tetromino {
            0.0
        } else if self.cleared_blocks == 0 {
            if self.lost_blocks > 0 {
                0.0
            } else {
                1.0
            }
        } else {
            let lost_ratio = self.lost_blocks as f32 / self.cleared_blocks as f32;

            1.0 - lost_ratio
        }
    }
}

struct Game {
    n_lanes: usize,
    n_rows: usize,
    stats: Stats,
    tetromino_colors: Vec<Handle<ColorMaterial>>,
    current_tetromino_blocks: HashSet<Entity>,
    current_tetromino_joints: Vec<Entity>,
    camera: Option<Entity>,
}

impl Game {
    fn floor_y(&self) -> f32 {
        -(self.n_rows as f32) * 0.5
    }

    fn left_wall_x(&self) -> f32 {
        -(self.n_lanes as f32) * 0.5
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            n_lanes: 10,
            n_rows: 20,
            stats: Stats::default(),
            tetromino_colors: vec![],
            current_tetromino_blocks: HashSet::new(),
            current_tetromino_joints: vec![],
            camera: None,
        }
    }
}

fn byte_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn setup_game(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut rapier_config: ResMut<RapierConfiguration>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // While we want our sprite to look ~40 px square, we want to keep the physics units smaller
    // to prevent float rounding problems. To do this, we set the scale factor in RapierConfiguration
    // and divide our sprite_size by the scale.
    rapier_config.scale = BLOCK_PX_SIZE;

    game.tetromino_colors = vec![
        materials.add(byte_rgb(0, 244, 243).into()),
        materials.add(byte_rgb(238, 243, 0).into()),
        materials.add(byte_rgb(177, 0, 254).into()),
        materials.add(byte_rgb(27, 0, 250).into()),
        materials.add(byte_rgb(252, 157, 0).into()),
        materials.add(byte_rgb(0, 247, 0).into()),
        materials.add(byte_rgb(255, 0, 0).into()),
    ];

    game.camera = Some(
        commands
            .spawn()
            .insert_bundle(OrthographicCameraBundle::new_2d())
            .id(),
    );

    setup_board(&mut commands, &*game, materials);

    // initial tetromino
    spawn_tetromino(&mut commands, &mut game);
}

#[derive(Clone, Copy, Debug)]
enum TetrominoKind {
    I,
    O,
    T,
    J,
    L,
    S,
    Z,
}

impl TetrominoKind {
    fn random() -> Self {
        match rand::thread_rng().gen_range(0..7) {
            0 => Self::I,
            1 => Self::O,
            2 => Self::T,
            3 => Self::J,
            4 => Self::L,
            5 => Self::S,
            _ => Self::Z,
        }
    }

    fn layout(&self) -> TetrominoLayout {
        match self {
            Self::I => TetrominoLayout {
                coords: [(1, 1), (1, 0), (1, -1), (1, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::O => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (0, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3), (1, 0)],
            },
            Self::T => TetrominoLayout {
                coords: [(0, 0), (1, 0), (2, 0), (1, -1)],
                joints: vec![(0, 1), (1, 2), (1, 3)],
            },
            Self::J => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (0, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::L => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (2, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::S => TetrominoLayout {
                coords: [(0, -1), (1, -1), (1, 0), (2, 0)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::Z => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (2, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
        }
    }
}

struct TetrominoLayout {
    coords: [(i32, i32); 4],
    joints: Vec<(usize, usize)>,
}

struct Block;

struct HealthBar {
    value: f32,
}

fn setup_board(commands: &mut Commands, game: &Game, mut materials: ResMut<Assets<ColorMaterial>>) {
    let floor_y = game.floor_y();

    // Add floor
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
            sprite: Sprite::new(Vec2::new(
                game.n_lanes as f32 * BLOCK_PX_SIZE,
                FLOOR_BLOCK_HEIGHT * BLOCK_PX_SIZE,
            )),
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            body_type: bevy_rapier2d::prelude::RigidBodyType::Static,
            position: [0.0, floor_y - (FLOOR_BLOCK_HEIGHT * 0.5)].into(),
            ..RigidBodyBundle::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(game.n_lanes as f32 * 0.5, FLOOR_BLOCK_HEIGHT * 0.5),
            ..ColliderBundle::default()
        })
        .insert(RigidBodyPositionSync::Discrete);

    // Add health bar
    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
            sprite: Sprite::new(Vec2::new(
                (game.n_lanes as f32 - 2.0) * BLOCK_PX_SIZE,
                BLOCK_PX_SIZE * HEALTH_BAR_HEIGHT,
            )),
            transform: Transform {
                translation: Vec3::new(
                    (game.left_wall_x() + 1.0) * BLOCK_PX_SIZE,
                    (floor_y - (FLOOR_BLOCK_HEIGHT / 2.0)) * BLOCK_PX_SIZE,
                    2.0,
                ),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.0, 1.0, 1.0),
            },
            ..Default::default()
        })
        .insert(HealthBar { value: 0.0 });
}

fn spawn_tetromino(commands: &mut Commands, game: &mut Game) {
    let kind = TetrominoKind::random();
    let TetrominoLayout { coords, joints } = kind.layout();

    let block_entities: Vec<Entity> = coords
        .iter()
        .map(|(x, y)| {
            let lane = (game.n_lanes as i32 / 2) - 1 + x;
            let row = game.n_rows as i32 - 1 + y;
            spawn_block(commands, game, kind, lane, row)
        })
        .collect();

    let joint_entities: Vec<Entity> = joints
        .iter()
        .map(|(i, j)| {
            let x_dir = coords[*j].0 as f32 - coords[*i].0 as f32;
            let y_dir = coords[*j].1 as f32 - coords[*i].1 as f32;

            let anchor_1 = Vec2::new(x_dir * 0.5, y_dir * 0.5).into();
            let anchor_2 = Vec2::new(x_dir * -0.5, y_dir * -0.5).into();

            commands
                .spawn()
                .insert_bundle((JointBuilderComponent::new(
                    BallJoint::new(anchor_1, anchor_2),
                    block_entities[*i],
                    block_entities[*j],
                ),))
                .id()
        })
        .collect();

    game.stats.generated_blocks += block_entities.len() as i32;

    game.current_tetromino_blocks = block_entities.into_iter().collect();
    game.current_tetromino_joints = joint_entities;
}

fn spawn_block(
    commands: &mut Commands,
    game: &Game,
    kind: TetrominoKind,
    lane: i32,
    row: i32,
) -> Entity {
    // x, y is the center of the block
    let x = game.left_wall_x() + lane as f32 + 0.5;
    let y = game.floor_y() + row as f32 + 0.5;

    // Game gets more difficult when this is lower:
    let linear_damping = 3.0;

    commands
        .spawn()
        .insert_bundle(SpriteBundle {
            material: game.tetromino_colors[kind as usize].clone(),
            sprite: Sprite::new(Vec2::new(BLOCK_PX_SIZE, BLOCK_PX_SIZE)),
            ..Default::default()
        })
        .insert_bundle(RigidBodyBundle {
            position: [x, y].into(),
            damping: RigidBodyDamping {
                linear_damping,
                angular_damping: 0.0,
            },
            ..RigidBodyBundle::default()
        })
        .insert_bundle(ColliderBundle {
            shape: ColliderShape::cuboid(0.5, 0.5),
            ..ColliderBundle::default()
        })
        .insert(RigidBodyPositionSync::Discrete)
        .insert(Block)
        .id()
}

fn tetromino_sleep_detection(
    mut commands: Commands,
    mut game: ResMut<Game>,
    block_query: Query<(Entity, &RigidBodyActivation, &RigidBodyPosition)>,
) {
    /*
    let all_blocks_sleeping = game.current_tetromino_blocks.iter().all(|block_entity| {
        block_query
            .get(*block_entity)
            .ok()
            .map(|(_, activation, _)| (activation.sleeping))
            .unwrap_or(false)
    });
    */
    let all_blocks_sleeping = true;

    if all_blocks_sleeping {
        for joint in &game.current_tetromino_joints {
            commands.entity(*joint).despawn();
        }

        clear_filled_rows(&mut commands, &mut game, block_query);

        if game.stats.health() > 0.0 {
            spawn_tetromino(&mut commands, &mut game);
        }
    }
}

fn clear_filled_rows(
    commands: &mut Commands,
    game: &mut Game,
    block_query: Query<(Entity, &RigidBodyActivation, &RigidBodyPosition)>,
) {
    let mut blocks_per_row: Vec<Vec<Entity>> = (0..game.n_rows).map(|_| vec![]).collect();

    let floor_y = game.floor_y();

    for (block_entity, activation, position) in block_query.iter() {
        // Only sleeping blocks count.. So disregard blocks "falling off"
        // that are in the row
        /*
        if !activation.sleeping {
            continue;
        }
        */

        let floor_distance = position.position.translation.y - floor_y;

        // The center of a block on the floor is 0.5 above the floor, so .floor() the number ;)
        let row = floor_distance.floor() as i32;

        if row >= 0 && row < game.n_rows as i32 {
            blocks_per_row[row as usize].push(block_entity);
        }
    }

    for row_blocks in blocks_per_row {
        if row_blocks.len() == game.n_lanes as usize {
            game.stats.cleared_blocks += game.n_lanes as i32;

            for block_entity in row_blocks {
                commands.entity(block_entity).despawn_recursive();
            }
        }
    }
}
