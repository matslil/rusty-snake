use rusty_engine::prelude::*;
use rand::prelude::*;
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Direction {
    UP,
    RIGHT,
    DOWN,
    LEFT,
}

struct Player {
    sprite: SpritePreset,
    labels: VecDeque<String>,
    max_len: usize,
    serial: usize,
    direction: Direction,
    head_pos: Vec2,
    control: Vec<KeyCode>,
    loose: bool,
}

const PLAYER_CONTROL: [[KeyCode; 2];4] = [
    [KeyCode::Q, KeyCode::W],
    [KeyCode::F, KeyCode::G],
    [KeyCode::U, KeyCode::I],
    [KeyCode::K, KeyCode::L],
];

impl Player {
    fn new(instance: usize) -> Self {
        const SPRITE_PRESETS: [SpritePreset; 4] = [
            SpritePreset::RollingBallBlueAlt,
            SpritePreset::RollingBallBlue,
            SpritePreset::RollingBallRedAlt,
            SpritePreset::RollingBallRed
        ];
        let player_label = format!("player{}.0", instance);
        let mut player = Player {
            sprite: SPRITE_PRESETS[instance],
            labels: VecDeque::new(),
            max_len: 4,
            serial: 1,
            direction: Direction::RIGHT,
            head_pos: Vec2 { x: instance as f32 * 20.0, y: 0.0 },
            control: PLAYER_CONTROL[instance].into(),
            loose: false,
        };
        player.labels.push_front(player_label);
        player
    }
}

struct GameState {
    obstacle_labels: VecDeque<String>,
    obstacle_idx: u32,
    obstacle_nr: u32,
    obstacle_max: u32,
    spawn_timer: Timer,
    speed_timer: Timer,
    player_move_timer: Timer,
    pill_timer: Timer,
    pill_idx: usize,
    speed: u32,
    player: Vec<Player>,
}

const PLAYER_MOVE_TIMER_START: f32 = 0.3;

const PLAYER_SCALE_TAIL: f32 = 0.2;
const PLAYER_SCALE_HEAD: f32 = 0.3;

impl Default for GameState {
    fn default() -> Self {
        GameState {
            spawn_timer: Timer::from_seconds(5.0, false),
            speed_timer: Timer::from_seconds(10.0, true),
            player_move_timer: Timer::from_seconds(PLAYER_MOVE_TIMER_START, false),
            pill_timer: Timer::from_seconds(1.0, false),
            pill_idx: 0,
            obstacle_labels: VecDeque::new(),
            obstacle_idx: 0,
            obstacle_nr: 0,
            obstacle_max: 5,
            speed: 1,
            player: Vec::new(),
        }
    }
}

pub fn start_game() {
    let mut game = Game::new();

    let state = GameState::default();

    game.window_settings(WindowDescriptor {
        title: "Snake".into(),
        ..Default::default()
    });

    game.audio_manager.play_music(MusicPreset::Classy8Bit, 0.1);

    game.add_logic(game_logic);
    game.run(state);
}

fn speed_to_spawn_timeout(initial: f32, speed: u32) -> f32 {
    let lower = initial / speed as f32;
    let higher = initial * 2.0 / speed as f32;
    thread_rng().gen_range(lower..higher)
}

fn new_direction(curr_dir: Direction, turn_left: bool) -> Direction {
    match curr_dir {
        Direction::UP    => if turn_left { Direction::LEFT } else { Direction::RIGHT },
        Direction::RIGHT => if turn_left { Direction::UP } else { Direction::DOWN },
        Direction::DOWN  => if turn_left { Direction::RIGHT } else { Direction::LEFT },
        Direction::LEFT  => if turn_left { Direction::DOWN } else { Direction::UP },
    }
}

fn new_position(engine: &Engine, pos: Vec2, dir: Direction) -> Vec2 {
    const DISTANCE: f32 = 10.0;
    let mut pos = match dir {
        Direction::UP    => Vec2 { x: pos.x,            y: pos.y + DISTANCE },
        Direction::RIGHT => Vec2 { x: pos.x + DISTANCE, y: pos.y },
        Direction::DOWN  => Vec2 { x: pos.x,            y: pos.y - DISTANCE },
        Direction::LEFT  => Vec2 { x: pos.x - DISTANCE, y: pos.y },
    };
    let max_x = engine.window_dimensions.x / 2.0;
    let max_y = engine.window_dimensions.y / 2.0;
    if pos.x > max_x {
        pos.x = -max_x;
    }
    if pos.x < -max_x {
        pos.x = max_x;
    }
    if pos.y > max_y {
        pos.y = -max_y;
    }
    if pos.y < -max_y {
        pos.y = max_y;
    }
    pos
}

fn game_logic(engine: &mut Engine, state: &mut GameState) {
    if state.speed_timer.tick(engine.delta).just_finished() {
        state.speed += 1;
    }
    let x = engine.window_dimensions.x / 2.0;
    let y = engine.window_dimensions.y / 2.0;
    if state.spawn_timer.tick(engine.delta).just_finished() {
        state.spawn_timer = Timer::from_seconds(speed_to_spawn_timeout(3.0, state.speed), false);

        let label = format!("obstacle{}", state.obstacle_idx);
        state.obstacle_idx += 1;
        let obstacle = engine.add_sprite(label.clone(), SpritePreset::RacingBarrelRed);
        state.obstacle_labels.push_front(label);
        obstacle.translation.x = thread_rng().gen_range(-(x+20.0)..(x-20.0));
        obstacle.translation.y = thread_rng().gen_range(-(y+20.0)..(y-20.0));
        obstacle.collision = true;
        state.obstacle_nr += 1;
        if state.obstacle_nr > state.obstacle_max {
            engine.sprites.remove(&state.obstacle_labels.pop_back().unwrap());
            state.obstacle_nr -= 1;
        }
    }

    if state.pill_timer.tick(engine.delta).just_finished() {
        state.pill_timer = Timer::from_seconds(speed_to_spawn_timeout(2.0, state.speed), false);

        let label = format!("pill{}", state.pill_idx);
        state.pill_idx += 1;

        let pill = engine.add_sprite(label, SpritePreset::RacingBarrelBlue);
        pill.translation.x = thread_rng().gen_range(-(x+20.0)..(x-20.0));
        pill.translation.y = thread_rng().gen_range(-(y+20.0)..(y-20.0));
        pill.collision = true;
    }

    for control in PLAYER_CONTROL {
        if engine.keyboard_state.just_pressed_any(&control) {
            let mut handled = false;
            for player in &mut state.player {
                if player.loose {
                    continue;
                }
                if control == player.control.as_slice() {
                    player.direction = new_direction(player.direction, engine.keyboard_state.pressed(control[0]));
                    handled = true;
                    break;
                }
            }
            if ! handled {
                state.player.push(Player::new(state.player.len()));
            }
        }
    }

    if state.player_move_timer.tick(engine.delta).just_finished() {
        state.player_move_timer = Timer::from_seconds(PLAYER_MOVE_TIMER_START / state.speed as f32, false);

        for (idx, player) in state.player.iter_mut().enumerate() {
            if player.loose {
                continue;
            }
            let new_head_label = format!("player{}.{}", idx, player.serial);
            player.serial += 1;
            player.head_pos = new_position(&engine, player.head_pos, player.direction);
            println!("New head: {}, pos: {}", new_head_label, player.head_pos);

            // Old head now becomes tail, rescale it
            if let Some(sprite) = engine.sprites.get_mut(player.labels.front().unwrap()) {
                sprite.scale = PLAYER_SCALE_TAIL;
            }

            let new_head = engine.add_sprite(new_head_label.clone(), player.sprite);
            new_head.translation = player.head_pos;
            new_head.collision = true;
            new_head.scale = PLAYER_SCALE_HEAD;
            player.labels.push_front(new_head_label);
            if player.labels.len() > player.max_len {
                engine.sprites.remove(&player.labels.pop_back().unwrap());
                println!("Removing one from tail");
            }
        }
    }

    // Handle collisions
    for event in engine.collision_events.drain(..) {
        if event.state.is_end() {
            continue;
        }

        if event.pair.one_starts_with("player") {
            let mut player_label;
            let mut colliding_with_label;
            if event.pair.0.starts_with("player") {
                player_label = event.pair.0;
                colliding_with_label = event.pair.1;
            } else {
                player_label = event.pair.1;
                colliding_with_label = event.pair.0;
            }
            let player = &mut state.player[(player_label.strip_prefix("player").unwrap().chars().nth(0).unwrap() as u8 - '0' as u8) as usize];

            if colliding_with_label.starts_with("pill") {
                player.max_len += 1;
                engine.sprites.remove(&colliding_with_label);
                engine.audio_manager.play_sfx(SfxPreset::Confirmation1, 0.2);
            }

            if colliding_with_label.starts_with("obstacle") {
                player.loose = true;
                engine.audio_manager.play_sfx(SfxPreset::Impact1, 0.2);
            }
        }
    }
}
