use macroquad::prelude::*;
use crate::map::{MapData, Life, Ladder};

/// Bot AI state for a single mob
#[derive(Debug, Clone)]
pub struct BotState {
    pub life_id: String,
    pub x: f32,
    pub y: f32,
    pub vx: f32,  // Horizontal velocity
    pub vy: f32,  // Vertical velocity
    pub on_ground: bool,
    pub facing_right: bool,
    pub move_timer: f32,
    pub move_duration: f32,
    pub move_direction: i32, // -1 = left, 0 = idle, 1 = right
    pub climbing: bool,
    pub current_ladder: Option<i32>, // Ladder ID if climbing
}

/// Fake player state for simulating other players
#[derive(Debug, Clone)]
pub struct FakePlayer {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub on_ground: bool,
    pub facing_right: bool,
    pub move_timer: f32,
    pub move_direction: i32,
    pub level: u32,
    pub animation_state: FakePlayerState,
    pub animation_timer: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FakePlayerState {
    Stand,
    Walk,
    Jump,
}

impl FakePlayer {
    pub fn new(name: &str, x: f32, y: f32, level: u32) -> Self {
        Self {
            name: name.to_string(),
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            on_ground: true,
            facing_right: rand::gen_range(0, 2) == 0,
            move_timer: rand::gen_range(1.0, 4.0),
            move_direction: 0,
            level,
            animation_state: FakePlayerState::Stand,
            animation_timer: 0.0,
        }
    }
}

impl BotState {
    pub fn new(life: &Life) -> Self {
        // Adjust spawn position to account for origin offset
        // The map coordinate is the reference point, but we need to position
        // the mob so its feet are on the platform, not its anchor point
        let adjusted_y = life.y as f32;

        Self {
            life_id: life.id.clone(),
            x: life.x as f32,
            y: adjusted_y,
            vx: 0.0,
            vy: 0.0,
            on_ground: true,
            facing_right: !life.flip,
            move_timer: rand::gen_range(1.0, 3.0),
            move_duration: 0.0,
            move_direction: 0,
            climbing: false,
            current_ladder: None,
        }
    }
}

/// Bot AI manager that updates all mob AI
pub struct BotAI {
    pub bot_states: Vec<BotState>,
    pub fake_players: Vec<FakePlayer>,
}

impl BotAI {
    pub fn new() -> Self {
        Self {
            bot_states: Vec::new(),
            fake_players: Vec::new(),
        }
    }

    /// Get all mob states for collision detection
    pub fn get_mobs(&self) -> &[BotState] {
        &self.bot_states
    }

    /// Initialize bot states from map life data (mobs only)
    pub fn initialize_from_map(&mut self, map: &MapData) {
        self.bot_states.clear();
        self.fake_players.clear();

        for life in &map.life {
            // Only create AI for mobs (type "m"), not NPCs (type "n")
            if life.life_type == "m" {
                let mut bot = BotState::new(life);

                // Find foothold for this mob using the specified foothold ID
                if life.foothold != 0 {
                    if let Some(fh) = map.footholds.iter().find(|f| f.id == life.foothold) {
                        // Check if the specified foothold is vertical (not walkable)
                        let dx = (fh.x2 - fh.x1).abs();
                        let dy = (fh.y2 - fh.y1).abs();
                        let is_vertical = dy > 0 && (dx as f32 / dy as f32) < 0.1 && dy > 10;
                        
                        if !is_vertical {
                            // Horizontal foothold - use it
                            bot.y = map.get_foothold_y_at(fh, bot.x);
                            bot.on_ground = true;
                        } else {
                            // Vertical foothold - find horizontal one below
                            if let Some((foothold_y, _fh)) = map.find_foothold_below(bot.x, bot.y) {
                                bot.y = foothold_y;
                                bot.on_ground = true;
                            }
                        }
                    } else {
                        // Foothold ID not found, find one below
                        if let Some((foothold_y, _fh)) = map.find_foothold_below(bot.x, bot.y) {
                            bot.y = foothold_y;
                            bot.on_ground = true;
                        }
                    }
                } else if let Some((foothold_y, _fh)) = map.find_foothold_below(bot.x, bot.y) {
                    bot.y = foothold_y;
                    bot.on_ground = true;
                }

                self.bot_states.push(bot);
            }
        }

        // Spawn fake players at random locations on the map
        let fake_player_names = ["xXSlayerXx", "MapleHero", "NightWalker", "DragonKnight", "IceMage"];
        let num_fake_players = 3.min(fake_player_names.len());
        
        for i in 0..num_fake_players {
            // Find a random spawn point on a horizontal foothold (not vertical)
            let horizontal_footholds: Vec<_> = map.footholds.iter()
                .filter(|fh| {
                    let dx = (fh.x2 - fh.x1).abs() as f32;
                    let dy = (fh.y2 - fh.y1).abs() as f32;
                    // Only use horizontal footholds (dx > dy or dx > 2.0)
                    dx > 2.0 && dx > dy
                })
                .collect();
            
            if !horizontal_footholds.is_empty() {
                let fh_idx = rand::gen_range(0, horizontal_footholds.len());
                let fh = horizontal_footholds[fh_idx];
                let spawn_x = rand::gen_range(fh.x1.min(fh.x2) as f32, fh.x1.max(fh.x2) as f32);
                // Player Y is at feet level (on the foothold)
                let spawn_y = map.get_foothold_y_at(fh, spawn_x);
                let level = rand::gen_range(10, 100);
                
                let mut fake_player = FakePlayer::new(fake_player_names[i], spawn_x, spawn_y, level);
                fake_player.on_ground = true;
                self.fake_players.push(fake_player);
            }
        }
    }

    /// Update all bots and fake players
    pub fn update(&mut self, dt: f32, map: &MapData) {
        for bot in &mut self.bot_states {
            Self::update_bot(bot, dt, map);
        }
        
        for fake_player in &mut self.fake_players {
            Self::update_fake_player(fake_player, dt, map);
        }
    }

    /// Update a fake player's AI
    fn update_fake_player(player: &mut FakePlayer, dt: f32, map: &MapData) {
        // Update movement timer
        player.move_timer -= dt;
        player.animation_timer += dt;

        if player.move_timer <= 0.0 {
            // Choose a new action
            let action = rand::gen_range(0, 10);
            if action < 3 {
                player.move_direction = -1;
                player.move_timer = rand::gen_range(1.0, 4.0);
            } else if action < 6 {
                player.move_direction = 1;
                player.move_timer = rand::gen_range(1.0, 4.0);
            } else if action < 8 {
                player.move_direction = 0;
                player.move_timer = rand::gen_range(0.5, 2.0);
            } else {
                // Jump
                if player.on_ground {
                    player.vy = -400.0;
                    player.on_ground = false;
                }
                player.move_timer = rand::gen_range(0.5, 1.5);
            }
        }

        // Apply movement
        let move_speed = 150.0;
        player.vx = (player.move_direction as f32) * move_speed;
        player.x += player.vx * dt;

        // Update facing direction
        if player.vx > 0.0 {
            player.facing_right = true;
        } else if player.vx < 0.0 {
            player.facing_right = false;
        }

        // Apply gravity
        let gravity = 800.0;
        player.vy += gravity * dt;
        player.y += player.vy * dt;

        // Check collision with footholds (player.y is feet position)
        // Use find_foothold_below to find ground beneath player
        if let Some((fh_y, _fh)) = map.find_foothold_below(player.x, player.y - 10.0) {
            if player.y >= fh_y && player.vy >= 0.0 {
                player.y = fh_y;
                player.vy = 0.0;
                player.on_ground = true;
            } else {
                player.on_ground = false;
            }
        } else {
            player.on_ground = false;
        }

        // Clamp to map bounds
        player.x = player.x.max(map.info.vr_left as f32).min(map.info.vr_right as f32);
        player.y = player.y.min(map.info.vr_bottom as f32);

        // Update animation state
        if !player.on_ground {
            player.animation_state = FakePlayerState::Jump;
        } else if player.vx.abs() > 0.1 {
            player.animation_state = FakePlayerState::Walk;
        } else {
            player.animation_state = FakePlayerState::Stand;
        }
    }

    /// Update a single bot's AI
    fn update_bot(bot: &mut BotState, dt: f32, map: &MapData) {
        // Find the life data for this bot
        let life = match map.life.iter().find(|l| l.id == bot.life_id) {
            Some(l) => l,
            None => return,
        };

        // If climbing, handle ladder logic
        if bot.climbing {
            Self::update_climbing(bot, dt, map, life);
            return;
        }

        // Update movement timer
        bot.move_timer -= dt;

        if bot.move_timer <= 0.0 {
            // Choose a new action
            Self::choose_new_action(bot, life);
        }

        // Apply movement based on current direction
        let base_speed = 50.0; // Slower than player for more natural mob movement
        bot.vx = (bot.move_direction as f32) * base_speed;

        // Apply horizontal movement
        bot.x += bot.vx * dt;

        // Clamp to spawn range if defined
        if life.rx0 != 0 || life.rx1 != 0 {
            let min_x = life.rx0.min(life.rx1) as f32;
            let max_x = life.rx0.max(life.rx1) as f32;

            if bot.x < min_x {
                bot.x = min_x;
                bot.move_direction = 0; // Stop if hitting boundary
                bot.move_timer = 0.0; // Force new action
            } else if bot.x > max_x {
                bot.x = max_x;
                bot.move_direction = 0;
                bot.move_timer = 0.0;
            }
        }

        // Check if near a ladder and randomly decide to climb
        if bot.on_ground && rand::gen_range(0.0, 1.0) < 0.05 * dt {
            if let Some(ladder) = Self::find_nearby_ladder(bot, map) {
                bot.climbing = true;
                bot.current_ladder = Some(ladder.id);
                bot.x = ladder.x as f32; // Snap to ladder x position
                bot.vx = 0.0;
                bot.vy = -30.0; // Start climbing up
                bot.on_ground = false;
                return;
            }
        }

        // Apply gravity
        let gravity = 800.0;
        bot.vy += gravity * dt;

        // Update vertical position
        bot.y += bot.vy * dt;

        // Check collision with footholds (bot.y is the mob's feet position)
        // Always try to find and snap to a foothold
        if let Some((fh_y, _)) = map.find_foothold_below(bot.x, bot.y + 50.0) {
            // Snap to foothold if close enough or falling
            if bot.y >= fh_y - 5.0 || bot.vy >= 0.0 {
                bot.y = fh_y;
                bot.vy = 0.0;
                bot.on_ground = true;
            }
        } else {
            bot.on_ground = false;
        }

        // Clamp to map bounds
        bot.x = bot.x.max(map.info.vr_left as f32).min(map.info.vr_right as f32);
        bot.y = bot.y.max(map.info.vr_top as f32).min(map.info.vr_bottom as f32);

        // Update facing direction based on movement
        if bot.vx > 0.0 {
            bot.facing_right = true;
        } else if bot.vx < 0.0 {
            bot.facing_right = false;
        }
    }

    /// Handle ladder climbing logic
    fn update_climbing(bot: &mut BotState, dt: f32, map: &MapData, _life: &Life) {
        // Find the ladder
        let ladder = match bot.current_ladder {
            Some(id) => map.ladders.iter().find(|l| l.id == id),
            None => {
                bot.climbing = false;
                return;
            }
        };

        if let Some(ladder) = ladder {
            // Move up or down on the ladder
            bot.y += bot.vy * dt;

            // Check if reached top or bottom of ladder
            let min_y = ladder.y1.min(ladder.y2) as f32;
            let max_y = ladder.y1.max(ladder.y2) as f32;

            if bot.y <= min_y {
                // Reached top - exit ladder
                bot.climbing = false;
                bot.current_ladder = None;
                bot.y = min_y;
                bot.vy = 0.0;
                bot.on_ground = true;
                bot.move_timer = 0.0; // Choose new action immediately
            } else if bot.y >= max_y {
                // Reached bottom - exit ladder
                bot.climbing = false;
                bot.current_ladder = None;
                bot.y = max_y;
                bot.vy = 0.0;
                bot.on_ground = true;
                bot.move_timer = 0.0;
            }

            // Random chance to exit ladder midway
            if rand::gen_range(0.0, 1.0) < 0.2 * dt {
                bot.climbing = false;
                bot.current_ladder = None;
                bot.vy = 0.0;
                bot.move_timer = 0.0;
            }
        } else {
            // Ladder not found, stop climbing
            bot.climbing = false;
            bot.current_ladder = None;
        }
    }

    /// Choose a new random action for the bot
    fn choose_new_action(bot: &mut BotState, _life: &Life) {
        let action = rand::gen_range(0, 10);

        if action < 4 {
            // 40% chance to move left
            bot.move_direction = -1;
            bot.move_duration = rand::gen_range(1.0, 3.0);
            bot.move_timer = bot.move_duration;
        } else if action < 8 {
            // 40% chance to move right
            bot.move_direction = 1;
            bot.move_duration = rand::gen_range(1.0, 3.0);
            bot.move_timer = bot.move_duration;
        } else {
            // 20% chance to idle
            bot.move_direction = 0;
            bot.move_duration = rand::gen_range(0.5, 2.0);
            bot.move_timer = bot.move_duration;
        }
    }

    /// Find a nearby ladder that the bot can climb
    fn find_nearby_ladder<'a>(bot: &BotState, map: &'a MapData) -> Option<&'a Ladder> {
        for ladder in &map.ladders {
            // Check if bot is near the ladder horizontally (within 20 pixels)
            if (bot.x - ladder.x as f32).abs() < 20.0 {
                // Check if bot is within vertical range of the ladder
                let min_y = ladder.y1.min(ladder.y2) as f32;
                let max_y = ladder.y1.max(ladder.y2) as f32;

                if bot.y >= min_y - 10.0 && bot.y <= max_y + 10.0 {
                    return Some(ladder);
                }
            }
        }
        None
    }

    /// Get bot state by life ID
    pub fn get_bot_state(&self, life_id: &str) -> Option<&BotState> {
        self.bot_states.iter().find(|b| b.life_id == life_id)
    }
}
