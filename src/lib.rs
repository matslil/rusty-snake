/*
 * Rewrite state.player to be an array instead.
 * Player could have a default, which makes it empty.
 * Make a function that can take &mut to a player and initialize it.
 * Use that when creating a new player.
 * Transform "loose" to a state which can be one of:
 * - WAITING
 * - PLAYING
 * - LOST
 *
 * Player score text label should be store in player struct.
 *
 * Finish implementation of new_position_rad().
 */

use rusty_engine::prelude::*;
use rand::prelude::*;
use std::collections::VecDeque;
use core::f32::consts::*;

const MAX_NR_PLAYERS: usize = 4;

/* For each player, what key is used for turn left and turn right */
const PLAYER_CONTROL: [[KeyCode; 2]; MAX_NR_PLAYERS] = [
    /*  Left         Right   */
    [KeyCode::Q, KeyCode::W], /* Player 0 */
    [KeyCode::F, KeyCode::G], /* Player 1 */
    [KeyCode::U, KeyCode::I], /* Player 2 */
    [KeyCode::K, KeyCode::L], /* Player 3 */
];

const PLAYER_SPRITE_PRESETS: [SpritePreset; MAX_NR_PLAYERS] = [
    SpritePreset::RollingBallBlueAlt,
    SpritePreset::RollingBallBlue,
    SpritePreset::RollingBallRedAlt,
    SpritePreset::RollingBallRed
];

/* Starting move speed for a new player, in seconds per move */
const PLAYER_MOVE_TIMER_START: f32 = 0.1;

/* Size of player tail */
const PLAYER_SCALE_TAIL: f32 = 0.2;

/* Size of player head */
const PLAYER_SCALE_HEAD: f32 = 0.3;

/* How many seconds to freeze a player when player lost, after freeze player will be removed */
const PLAYER_LOOSE_TIMEOUT: f32 = 5.0;

/* How many virtual pixels to move player each time */
const PLAYER_MOVE_DISTANCE: f32 = 10.0;

const PLAYER_STARTING_MAX_LEN: usize = 4;

const PILL_SPAWN_INTERVAL: f32 = 3.0;

#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
enum Direction {
    UP,
    RIGHT,
    DOWN,
    LEFT,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum PlayerState {
    WAITING,
    PLAYING,
    LOST,
}

struct Player {
    /* Static for each instance */
    idx: usize,
    sprite: SpritePreset,
    head_label: String,
    score_label: String,
    control: Vec<KeyCode>,
    starting_direction: Direction,
    starting_position: Vec2,

    /* Variable */
    labels: VecDeque<String>,
    max_len: usize,
    serial: usize,
    direction: Direction,
    state: PlayerState,
    loose_timeout: Timer,
}

impl Player {
    fn new(instance: usize) -> Self {
        Player {
            idx: instance,
            sprite: PLAYER_SPRITE_PRESETS[instance],
            head_label: format!("player-head{}", instance),
            score_label: format!("player-score{}", instance),
            control: PLAYER_CONTROL[instance].into(),
            starting_direction: Direction::RIGHT,
            starting_position: Vec2 { x: instance as f32 * 50.0, y: instance as f32 * 50.0 },
            labels: VecDeque::new(),
            max_len: PLAYER_STARTING_MAX_LEN,
            serial: 0,
            direction: Direction::RIGHT,
            state: PlayerState::WAITING,
            loose_timeout: Timer::from_seconds(PLAYER_LOOSE_TIMEOUT, false),
        }
    }

    fn lost(self: &mut Self) {
        self.state = PlayerState::LOST;
    }

    fn is_playing(self: &Self) -> bool {
        self.state == PlayerState::PLAYING
    }

    fn is_waiting(self: &Self) -> bool {
        self.state == PlayerState::WAITING
    }

    fn has_lost(self: &Self) -> bool {
        self.state == PlayerState::LOST
    }

    /* Make player into same state as when new() was run */
    fn deactivate(self: &mut Self) {
        self.labels.truncate(0);
        self.max_len = PLAYER_STARTING_MAX_LEN;
        self.serial = 0;
        self.direction = self.starting_direction;
        self.state = PlayerState::WAITING;
        self.loose_timeout = Timer::from_seconds(PLAYER_LOOSE_TIMEOUT, false);
    }

    fn activate(self: &mut Self) {
        self.state = PlayerState::PLAYING;
    }
}

#[derive(Debug, Clone)]
struct Obstacle {
    label: String,
    speed: Vec2, /* Virtual pixels in x and y per move */
}

impl Obstacle {
    const MIN_SCALE: f32 = 0.2;
    const MAX_SCALE: f32 = 1.2;

    fn new(engine: &mut Engine, idx: usize) -> Obstacle{
        let x = engine.window_dimensions.x / 2.0;
        let y = engine.window_dimensions.y / 2.0;
        let scale = thread_rng().gen_range(Obstacle::MIN_SCALE..Obstacle::MAX_SCALE);
        let obstacle = Obstacle {
            label: format!("obstacle{}", idx),
            speed: Vec2 {
                x: thread_rng().gen_range(-(2.0/scale)..(2.0/scale)),
                y: thread_rng().gen_range(-(2.0/scale)..(2.0/scale)),
            },
        };

        println!("Obstacle::new() -> {:?}", obstacle);
        let obstacle_sprite = engine.add_sprite(obstacle.label.clone(), SpritePreset::RacingBarrelRed);
        obstacle_sprite.translation = Vec2{x: thread_rng().gen_range((-x + 20.0)..(x - 20.0)), y: thread_rng().gen_range((-y + 20.0)..(y - 20.0))};
        obstacle_sprite.collision = true;
        obstacle_sprite.scale = scale;
        obstacle
    }

    fn bounce(self: &mut Obstacle, other: &mut Obstacle, self_sprite: &Sprite, other_sprite: &Sprite) {
        let x = self_sprite.translation.x - other_sprite.translation.x;
        let y = self_sprite.translation.y - other_sprite.translation.y;
        let d = x * x + y * y;

        let u1 = (self.speed.x * x + self.speed.y * y) / d;
        let u2 = (x * self.speed.y - y * self.speed.x) / d;
        let u3 = (other.speed.x * x + other.speed.y * y) / d;
        let u4 = (x * other.speed.y - y * other.speed.x) / d;

        let mm = self_sprite.scale + other_sprite.scale;
        let vu3 = (self_sprite.scale - other_sprite.scale) / mm * u1 + (2.0 * other_sprite.scale) / mm * u3;
        let vu1 = (other_sprite.scale - self_sprite.scale) / mm * u3 + (2.0 * self_sprite.scale) / mm * u1;

        other.speed.x = x * vu1 - y * u4;
        other.speed.y = y * vu1 + x * u4;
        self.speed.x = x * vu3 - y * u2;
        self.speed.y = y * vu3 + x * u2;
    }

    fn do_move(self: &mut Self, engine: &mut Engine) {
        let max_x = engine.window_dimensions.x / 2.0;
        let max_y = engine.window_dimensions.y / 2.0;
        let self_sprite = engine.sprites.get_mut(&self.label).unwrap();
        let mut new_pos = Vec2 {
            x: self_sprite.translation.x + self.speed.x,
            y: self_sprite.translation.y + self.speed.y,
        };
        if new_pos.x > max_x {
            new_pos.x = -max_x;
        }
        if new_pos.x < -max_x {
            new_pos.x = max_x;
        }
        if new_pos.y > max_y {
            new_pos.y = -max_y;
        }
        if new_pos.y < -max_y {
            new_pos.y = max_y;
        }
        self_sprite.translation = new_pos;
    }
}

const OBSTACLE_MOVE_INTERVAL: f32 = 0.1;
const OBSTACLE_SPAWN_INTERVAL: f32 = 6.0;

struct GameState {
    /* Some initialization must be done first call to game logic */
    first_iteration: bool,

    /* Moving obstacles */
    obstacles: Vec<Obstacle>,

    obstacle_serial: usize,

    /* When it's time to add one more obstacle */
    obstacle_next_timer: Timer,

    /* Interval when to move obstacles */
    obstacle_move_timer: Timer,

    /* When it's time for a player snake to move */
    player_move_timer: Timer,

    /* When to add a pill */
    pill_timer: Timer,

    /* Nr of pills added, to create unique pill names */
    pill_idx: usize,

    /* Players */
    player: [Player; MAX_NR_PLAYERS],
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            first_iteration: true,
            obstacles: Vec::new(),
            obstacle_serial: 0,
            obstacle_next_timer: Timer::from_seconds(OBSTACLE_SPAWN_INTERVAL, true),
            obstacle_move_timer: Timer::from_seconds(OBSTACLE_MOVE_INTERVAL, true),
            player_move_timer: Timer::from_seconds(PLAYER_MOVE_TIMER_START, false),
            pill_timer: Timer::from_seconds(PILL_SPAWN_INTERVAL, true),
            pill_idx: 0,
            player: [
                Player::new(0),
                Player::new(1),
                Player::new(2),
                Player::new(3),
            ],
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

fn new_direction(curr_dir: Direction, turn_left: bool) -> Direction {
    match curr_dir {
        Direction::UP    => if turn_left { Direction::LEFT } else { Direction::RIGHT },
        Direction::RIGHT => if turn_left { Direction::UP } else { Direction::DOWN },
        Direction::DOWN  => if turn_left { Direction::RIGHT } else { Direction::LEFT },
        Direction::LEFT  => if turn_left { Direction::DOWN } else { Direction::UP },
    }
}

fn new_position(engine: &Engine, pos: Vec2, dir: Direction, speed: f32) -> Vec2 {
    let mut new_pos = match dir {
        Direction::UP    => Vec2 { x: pos.x,            y: pos.y + speed },
        Direction::RIGHT => Vec2 { x: pos.x + speed, y: pos.y },
        Direction::DOWN  => Vec2 { x: pos.x,            y: pos.y - speed },
        Direction::LEFT  => Vec2 { x: pos.x - speed, y: pos.y },
    };
    let max_x = engine.window_dimensions.x / 2.0;
    let max_y = engine.window_dimensions.y / 2.0;
    if new_pos.x > max_x {
        new_pos.x = -max_x;
    }
    if new_pos.x < -max_x {
        new_pos.x = max_x;
    }
    if new_pos.y > max_y {
        new_pos.y = -max_y;
    }
    if new_pos.y < -max_y {
        new_pos.y = max_y;
    }
    new_pos
}

fn game_logic(engine: &mut Engine, state: &mut GameState) {
    if state.first_iteration {
        let nr_obstacles = 3;

        println!("Nr obstacles: {}", nr_obstacles);

        for _ in 0..nr_obstacles {
        }

        state.first_iteration = false;
    }

    let x = engine.window_dimensions.x / 2.0;
    let y = engine.window_dimensions.y / 2.0;

    if state.obstacle_move_timer.tick(engine.delta).just_finished() {
        for obstacle in state.obstacles.iter_mut() {
            obstacle.do_move(engine);
        }
    }

    if state.obstacle_next_timer.tick(engine.delta).just_finished() {
        state.obstacle_next_timer = Timer::from_seconds(thread_rng().gen_range(2.0..10.0), false);
        state.obstacles.push(Obstacle::new(engine, state.obstacle_serial));
        state.obstacle_serial += 1;
    }

    // Check if it's time to add a pill
    if state.pill_timer.tick(engine.delta).just_finished() {
        let label = format!("pill{}", state.pill_idx);
        state.pill_idx += 1;

        let pill = engine.add_sprite(label, SpritePreset::RacingBarrelBlue);
        pill.translation.x = thread_rng().gen_range(-(x+20.0)..(x-20.0));
        pill.translation.y = thread_rng().gen_range(-(y+20.0)..(y-20.0));
        pill.collision = true;
    }

    // Check if it's time for players to move
    if state.player_move_timer.tick(engine.delta).just_finished() {
        state.player_move_timer = Timer::from_seconds(PLAYER_MOVE_TIMER_START, false);

        for (idx, player) in state.player.iter_mut().enumerate() {
            if ! player.is_playing() {
                continue;
            }

            let head_old_pos = engine.sprites.get(&player.head_label).unwrap().translation;
            let head_new_pos = new_position(&engine, head_old_pos, player.direction, PLAYER_MOVE_DISTANCE);
            let head_sprite = engine.sprites.get_mut(&player.head_label).unwrap();
            head_sprite.translation = head_new_pos;

            let tail_label = format!("player-tail{}.{}", idx, player.serial);
            player.serial += 1;
            let add_tail = engine.add_sprite(tail_label.clone(), player.sprite);
            add_tail.translation = head_old_pos;
            add_tail.collision = true;
            add_tail.scale = PLAYER_SCALE_TAIL;
            player.labels.push_front(tail_label);
            if player.labels.len() > player.max_len {
                engine.sprites.remove(&player.labels.pop_back().unwrap());
            }
        }
    }

    // Check for key-presses, includes detecting a new player
    for player in &mut state.player {
        if engine.keyboard_state.just_pressed_any(&player.control) {
            if player.is_playing() {
                player.direction = new_direction(player.direction, engine.keyboard_state.pressed(player.control[0]));
            } else if player.is_waiting() {
                player.activate();
                let _ = engine.texts.remove(&player.score_label);
                let head = engine.add_sprite(&player.head_label, player.sprite);
                head.translation = player.starting_position;
                head.collision = true;
                head.scale = PLAYER_SCALE_HEAD;
            }
        }
    }

    for player in state.player.iter_mut() {
        if ! player.has_lost() {
            continue;
        }

        if player.loose_timeout.tick(engine.delta).just_finished() {
            let player_text = engine.add_text(player.score_label.clone(), format!("Player {}: {} points", player.idx, player.labels.len() * 10));
            player_text.translation = Vec2::new(-x + 100.0 + (player.idx as f32 * 100.0), y - 50.0);
            player_text.scale = 0.4;
            engine.sprites.remove(&player.head_label);
            for label in &player.labels {
                engine.sprites.remove(label);
            }
            player.deactivate();
        }
    }

    // Handle collisions
    for event in engine.collision_events.drain(..) {
        if event.state.is_end() {
            continue;
        }

        if event.pair.one_starts_with("player-head") {
            println!("Collision with player: {:?}", event.pair);
            // Figure out which side is the player and which is what the player collided with
            let player_label;
            let colliding_with_label;
            if event.pair.0.starts_with("player-head") {
                player_label = event.pair.0;
                colliding_with_label = event.pair.1;
            } else {
                player_label = event.pair.1;
                colliding_with_label = event.pair.0;
            }

            // Get player object based on label name
            let player = &mut state.player[(player_label.strip_prefix("player-head").unwrap().chars().nth(0).unwrap() as u8 - '0' as u8) as usize];

            // If pill, then eat it, otherwise loose
            if colliding_with_label.starts_with("pill") {
                player.max_len += 1;
                engine.sprites.remove(&colliding_with_label);
                engine.audio_manager.play_sfx(SfxPreset::Confirmation1, 0.2);
            } else {
                player.lost();
                engine.audio_manager.play_sfx(SfxPreset::Impact1, 0.2);
                println!("{} lost", player_label);
            }
        } else if event.pair.0.starts_with("obstacle") && event.pair.1.starts_with("obstacle") {
            let mut obstacle1: Option<&mut Obstacle> = Option::None;
            let mut obstacle2: Option<&mut Obstacle> = Option::None;
            for obstacle in state.obstacles.iter_mut() {
                if obstacle.label == event.pair.0 {
                    obstacle1 = Some(obstacle);
                } else if obstacle.label == event.pair.1 {
                    obstacle2 = Some(obstacle);
                }
            }
            if obstacle1.is_some() && obstacle2.is_some() {
                let this = obstacle1.unwrap();
                let other = obstacle2.unwrap();
                let this_sprite = engine.sprites.get(&this.label).unwrap();
                let other_sprite = engine.sprites.get(&other.label).unwrap();
                this.bounce(other, this_sprite, other_sprite);
            }
        }
    }
}
