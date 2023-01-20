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
use std::collections::HashMap;

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

struct Object {
    label: String,
    pos: Vec2,
    speed: Vec2,
    scale: f32,
}

impl Object {
    fn bounce(self: &mut Self, other: &mut Self) {
        let x = self.pos.x - other.pos.x;
        let y = self.pos.y - other.pos.y;
        let d = x * x + y * y;

        let u1 = (self.speed.x * x + self.speed.y * y) / d;
        let u2 = (x * self.speed.y - y * self.speed.x) / d;
        let u3 = (other.speed.x * x + other.speed.y * y) / d;
        let u4 = (x * other.speed.y - y * other.speed.x) / d;

        let mm = self.scale + other.scale;
        let vu3 = (self.scale - other.scale) / mm * u1 + (2.0 * other.scale) / mm * u3;
        let vu1 = (other.scale - self.scale) / mm * u3 + (2.0 * self.scale) / mm * u1;

        other.speed.x = x * vu1 - y * u4;
        other.speed.y = y * vu1 + x * u4;
        self.speed.x = x * vu3 - y * u2;
        self.speed.y = y * vu3 + x * u2;
    }

    fn do_move(self: &mut Self, pos_max: Vec2) {
        self.pos.x += self.speed.x;
        self.pos.y += self.speed.y;

        if self.pos.x > pos_max.x {
            self.pos.x = -pos_max.x;
        }
        if self.pos.x < -pos_max.x {
            self.pos.x = pos_max.x;
        }
        if self.pos.y > pos_max.y {
            self.pos.y = -pos_max.y;
        }
        if self.pos.y < -pos_max.y {
            self.pos.y = pos_max.y;
        }
    }
}

const OBSTACLE_MOVE_INTERVAL: f32 = 0.1;
const OBSTACLE_SPAWN_INTERVAL: f32 = 6.0;

struct GameState {
    pos_max: Vec2,

    /* Some initialization must be done first call to game logic */
    first_iteration: bool,

    /* Moving obstacles */
    objects: HashMap<String, Object>,

    object_serial: usize,

    /* When it's time to add one more obstacle */
    obstacle_spawn_timer: Timer,

    /* Interval when to move obstacles */
    obstacle_move_timer: Timer,

    /* When to add a pill */
    pill_spawn_timer: Timer,


    /* When it's time for a player snake to move */
    player_move_timer: Timer,

    /* Players */
    player: [Player; MAX_NR_PLAYERS],
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            pos_max: Vec2 {x: 0.0, y: 0.0},
            first_iteration: true,
            objects: HashMap::new(),
            object_serial: 0,
            obstacle_spawn_timer: Timer::from_seconds(OBSTACLE_SPAWN_INTERVAL, true),
            obstacle_move_timer: Timer::from_seconds(OBSTACLE_MOVE_INTERVAL, true),
            player_move_timer: Timer::from_seconds(PLAYER_MOVE_TIMER_START, true),
            pill_spawn_timer: Timer::from_seconds(PILL_SPAWN_INTERVAL, true),
            player: [
                Player::new(0),
                Player::new(1),
                Player::new(2),
                Player::new(3),
            ],
        }
    }
}

impl GameState {
    fn add_obstacle(self: &mut GameState, engine: &mut Engine) {
        /* Minimum scale, also used as mass */
        const MIN_SCALE: f32 = 0.2;

        /* Maximum scale, also used as mass */
        const MAX_SCALE: f32 = 1.2;

        /* Speed for scale 1.0 in virtual pixels per move */
        const MAX_BASE_SPEED: f32 = 2.5;

        let scale = thread_rng().gen_range(MIN_SCALE..MAX_SCALE);
        let obstacle = Object {
            label: format!("object-obstacle{}", self.object_serial),
            pos: Vec2 {
                x: 0.0,
                y: 0.0,
            },
            speed: Vec2 {
                x: thread_rng().gen_range(-(MAX_BASE_SPEED/scale)..(MAX_BASE_SPEED/scale)),
                y: thread_rng().gen_range(-(MAX_BASE_SPEED/scale)..(MAX_BASE_SPEED/scale)),
            },
            scale,
        };
        self.object_serial += 1;
        let obstacle_sprite = engine.add_sprite(obstacle.label.clone(), SpritePreset::RacingBarrelRed);
        obstacle_sprite.translation = obstacle.pos;
        obstacle_sprite.scale = obstacle.scale;
        obstacle_sprite.collision = true;
        self.objects.insert(obstacle.label.clone(), obstacle);
    }

    fn add_pill(self: &mut GameState, engine: &mut Engine) {
        const MIN_SCALE: f32 = 0.2;
        const MAX_SCALE: f32 = 1.2;
        let pill = Object {
            label: format!("object-pill{}", self.object_serial),
            pos: Vec2 {
                x: thread_rng().gen_range(-(self.pos_max.x+20.0)..(self.pos_max.x-20.0)),
                y: thread_rng().gen_range(-(self.pos_max.y+20.0)..(self.pos_max.y-20.0)),
            },
            speed: Vec2 {
                x: 0.0,
                y: 0.0,
            },
            scale: thread_rng().gen_range(MIN_SCALE..MAX_SCALE),
        };
        self.object_serial += 1;
        let pill_sprite = engine.add_sprite(pill.label.clone(), SpritePreset::RacingBarrelBlue);
        pill_sprite.translation = pill.pos;
        pill_sprite.scale = pill.scale;
        pill_sprite.collision = true;
        self.objects.insert(pill.label.clone(), pill);
    }

    fn move_object(self: &mut GameState, engine: &mut Engine, object: &mut Object)
    {
        let object_sprite = engine.sprites.get_mut(&object.label).unwrap();
        object.do_move(self.pos_max);
        object_sprite.translation = object.pos;
    }

    fn object_collision(self: &mut GameState, objects: &CollisionPair) {
        let object1 = self.objects.get_mut(&objects.0).unwrap();
        let object2 = self.objects.get_mut(&objects.1).unwrap();
        object1.bounce(object2);
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
        state.pos_max = Vec2 {
            x: engine.window_dimensions.x / 2.0,
            y: engine.window_dimensions.y / 2.0,
        };
        state.first_iteration = false;
    }

    if state.obstacle_move_timer.tick(engine.delta).just_finished() {
        for object in state.objects.iter_mut() {
            object.1.do_move(state.pos_max);
        }
    }

    if state.obstacle_spawn_timer.tick(engine.delta).just_finished() {
        state.add_obstacle(engine);
    }

    // Check if it's time to add a pill
    if state.pill_spawn_timer.tick(engine.delta).just_finished() {
        state.add_pill(engine);
    }

    // Check if it's time for players to move
    if state.player_move_timer.tick(engine.delta).just_finished() {

        for player in state.player.iter_mut() {
            if ! player.is_playing() {
                continue;
            }

            let head_old_pos = engine.sprites.get(&player.head_label).unwrap().translation;
            let head_new_pos = new_position(&engine, head_old_pos, player.direction, PLAYER_MOVE_DISTANCE);
            let head_sprite = engine.sprites.get_mut(&player.head_label).unwrap();
            head_sprite.translation = head_new_pos;

            let tail_label = format!("player-tail{}.{}", player.idx, player.serial);
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
            player_text.translation = Vec2::new(-state.pos_max.x + 100.0 + (player.idx as f32 * 100.0), state.pos_max.y - 50.0);
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
            if colliding_with_label.starts_with("object-pill") {
                player.max_len += 1;
                engine.sprites.remove(&colliding_with_label);
                engine.audio_manager.play_sfx(SfxPreset::Confirmation1, 0.2);
            } else {
                player.lost();
                engine.audio_manager.play_sfx(SfxPreset::Impact1, 0.2);
                println!("{} lost", player_label);
            }
        } else if event.pair.0.starts_with("object") && event.pair.1.starts_with("object") {
            state.object_collision(&event.pair);
        }
    }
}
