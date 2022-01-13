use backroll_transport_steam::*;
use bevy::tasks::IoTaskPool;
use bevy::{core::FixedTimestep, prelude::*};
use bevy_backroll::{backroll::*, *};
use bytemuck::{Pod, Zeroable};
use std::ops::Deref;
use steamworks::{Client, SteamId};

pub type P2PSession = bevy_backroll::backroll::P2PSession<BackrollConfig>;

pub struct BackrollConfig;

#[macro_use]
extern crate bitflags;

#[derive(Clone, Component)]
pub struct Player {
    //position: Vec2,
    //velocity: Vec2,
    //size: Vec2,
    handle: PlayerHandle, // the network id
}

bitflags! {
    #[derive(Default, Pod, Zeroable)]
    #[repr(C)]
    pub struct PlayerInputFrame: u32 {
        // bit shift the stuff in the input struct
        const UP = 1<<0;
        const DOWN = 1<<1;
        const LEFT = 1<<2;
        const RIGHT = 1<<3;
    }
}

impl Config for BackrollConfig {
    type Input = PlayerInputFrame;
    type State = GameState;
}

#[derive(Clone, PartialEq, Hash)]
pub struct GameState {}

const MATCH_UPDATE_LABEL: &str = "MATCH_UPDATE";

const DELTA_TIME: f32 = 1.0 / 60.0; // in ms

pub struct OurBackrollPlugin;

impl Plugin for OurBackrollPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(BackrollPlugin::<BackrollConfig>::default())
            .with_rollback_run_criteria::<BackrollConfig, _>(
                FixedTimestep::step(DELTA_TIME.into()).with_label(MATCH_UPDATE_LABEL),
            )
            .with_input_sampler_system::<BackrollConfig, _>(sample_input.system())
            .with_world_save_system::<BackrollConfig, _>(save_world.system())
            .with_world_load_system::<BackrollConfig, _>(load_world.system());
    }
}

struct StartupNetworkConfig {
    client: usize,
    bind: Client,
    remote: SteamId,
}

fn sample_input(
    _handle: In<PlayerHandle>,
    keyboard_input: Res<Input<KeyCode>>,
) -> PlayerInputFrame {
    let mut local_input = PlayerInputFrame::empty();

    // local input handling
    {
        if keyboard_input.pressed(KeyCode::Left) {
            local_input.insert(PlayerInputFrame::LEFT);
            println!("Left");
        } else if keyboard_input.pressed(KeyCode::Right) {
            local_input.insert(PlayerInputFrame::RIGHT);
            println!("Right");
        }

        if keyboard_input.pressed(KeyCode::Up) {
            local_input.insert(PlayerInputFrame::UP);
            println!("Up");
        } else if keyboard_input.pressed(KeyCode::Down) {
            local_input.insert(PlayerInputFrame::DOWN);
            println!("Down");
        }
    }

    local_input
}

fn save_world() -> GameState {
    //println!("Save da world");
    GameState {}
}

fn load_world(_state: In<GameState>) {
    //println!("Load da world");
}

fn setup_game(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn spawn_players(mut commands: Commands, config: Res<StartupNetworkConfig>, pool: Res<IoTaskPool>) {
    let socket = SteamP2PManager::bind(pool.deref().deref().clone(), config.bind.clone());
    let peer = socket.connect(SteamConnectionConfig::unbounded(config.remote));

    commands.insert_resource(socket);

    let mut builder = backroll::P2PSession::<BackrollConfig>::build();

    commands
        .spawn_bundle(SpriteBundle {
            // sprite: Sprite::default(),
            ..Default::default()
        })
        // make sure to clone the player handles for reference stuff
        .insert(if config.client == 0 {
            // set up local player
            Player {
                handle: builder.add_player(backroll::Player::Local),
            }
        } else {
            // set up remote player
            Player {
                // make sure to clone the remote peer for reference stuff
                handle: builder.add_player(backroll::Player::Remote(peer.clone())),
            }
        });

    commands
        .spawn_bundle(SpriteBundle {
            // sprite: Sprite::new(Vec2::new(10.0, 10.0)),
            ..Default::default()
        })
        .insert(if config.client == 1 {
            // set up local player
            Player {
                handle: builder.add_player(backroll::Player::Local),
            }
        } else {
            // set up remote player
            Player {
                handle: builder.add_player(backroll::Player::Remote(peer)),
            }
        });

    commands.start_backroll_session(builder.start(pool.deref().deref().clone()).unwrap());
}

fn player_movement(
    keyboard_input: Res<GameInput<PlayerInputFrame>>,
    mut player_positions: Query<(&mut Transform, &Player)>,
) {
    for (mut transform, player) in player_positions.iter_mut() {
        let input = keyboard_input.get(player.handle).unwrap();
        if input.contains(PlayerInputFrame::LEFT) {
            transform.translation.x -= 2.;
        }
        if input.contains(PlayerInputFrame::RIGHT) {
            transform.translation.x += 2.;
        }
        if input.contains(PlayerInputFrame::DOWN) {
            transform.translation.y -= 2.;
        }
        if input.contains(PlayerInputFrame::UP) {
            transform.translation.y += 2.;
        }
    }
}

fn start_app(player_num: usize) {
    let (client, _single) = Client::init().unwrap();
    let remote_addr = SteamId::from_raw(76561199234689348);

    let bind_addr = client;

    App::new()
        .add_startup_system(setup_game.system())
        .add_startup_stage("game_setup", SystemStage::single(spawn_players.system()))
        .add_plugins(DefaultPlugins)
        .add_plugin(OurBackrollPlugin)
        .insert_resource(StartupNetworkConfig {
            client: player_num,
            bind: bind_addr,
            remote: remote_addr,
        })
        .with_rollback_system::<BackrollConfig, _, _>(player_movement.system())
        .run();
}
fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    if let Some(player_num) = args.next() {
        println!("{}", player_num);
        start_app(player_num.parse().unwrap());
    }
}
