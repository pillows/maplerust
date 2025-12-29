use crate::map::data::{Life, MapData};

/// Mob movement states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobStateType {
    Idle,
    Patrol,
    Chase,
    Jump,
    Attack,
    KnockedBack,
}

/// Runtime state for a mob (separate from Life which is static spawn data)
#[derive(Debug, Clone)]
pub struct MobState {
    pub life_index: usize,           // Index into map.life vector
    pub x: f32,                      // Current X position (f32 for smooth movement)
    pub y: f32,                      // Current Y position
    pub velocity_x: f32,             // Horizontal velocity
    pub velocity_y: f32,             // Vertical velocity (for jumping/falling)
    pub state: MobStateType,         // Current state
    pub direction: f32,              // -1.0 for left, 1.0 for right
    
    // Movement parameters
    pub base_speed: f32,             // Base movement speed
    pub chase_speed: f32,             // Speed when chasing player
    pub current_speed: f32,           // Current effective speed (with variations)
    
    // Patrol bounds
    pub patrol_left: f32,
    pub patrol_right: f32,
    pub current_foothold_id: i32,
    
    // Spawn tracking (for leashing)
    pub spawn_x: f32,
    pub spawn_y: f32,
    pub spawn_foothold_id: i32,
    
    // State timers
    pub state_timer: f32,             // Time in current state
    pub idle_duration: f32,           // How long to idle (randomized)
    pub chase_timeout: f32,           // When to give up chase
    pub detection_timer: f32,         // Timer for detection checks
    pub speed_variation_timer: f32,   // Timer for speed variation
    
    // Aggro system
    pub aggro: bool,
    pub aggro_range: f32,
    pub max_leash_distance: f32,
    pub last_player_x: f32,
    pub last_player_y: f32,
    
    // Jump physics
    pub jump_velocity: f32,
    pub on_ground: bool,
    
    // Natural movement
    pub hesitation_timer: f32,        // Timer before jumping at edge
    pub speed_multiplier: f32,        // Current speed variation (0.9-1.1)
}

impl MobState {
    pub fn new(life: &Life, map: &MapData) -> Self {
        let x = life.x as f32;
        
        // Find the foothold this mob is on and calculate Y position
        let foothold = map.footholds.iter().find(|fh| fh.id == life.foothold);
        let y = if let Some(fh) = foothold {
            // Calculate Y position on the foothold at spawn X
            let dx = fh.x2 - fh.x1;
            let dy = fh.y2 - fh.y1;
            let ix = x as i32;
            
            if dx != 0 {
                (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
            } else {
                fh.y1 as f32
            }
        } else {
            life.y as f32
        };
        
        // Determine patrol bounds based on foothold or spawn area
        let (patrol_left, patrol_right) = if let Some(fh) = foothold {
            let fh_left = fh.x1.min(fh.x2) as f32;
            let fh_right = fh.x1.max(fh.x2) as f32;
            let margin = 20.0;
            (
                fh_left + margin,
                fh_right - margin,
            )
        } else {
            let patrol_range = 200.0;
            (x - patrol_range, x + patrol_range)
        };
        
        // Randomize initial idle duration (1-3 seconds)
        let idle_duration = 1.0 + (life.x as f32 % 200.0) / 100.0; // Use spawn X as seed
        
        // Start moving in a direction based on initial flip
        let direction = if life.flip { -1.0 } else { 1.0 };
        
        // Mob type parameters (can be customized per mob ID later)
        let base_speed = 60.0;
        let chase_speed = base_speed * 1.4;
        let aggro_range = 200.0;
        let max_leash_distance = 800.0;
        
        Self {
            life_index: 0,
            x,
            y,
            velocity_x: 0.0,
            velocity_y: 0.0,
            state: MobStateType::Idle,
            direction,
            base_speed,
            chase_speed,
            current_speed: base_speed,
            patrol_left,
            patrol_right,
            current_foothold_id: life.foothold,
            spawn_x: x,
            spawn_y: y,
            spawn_foothold_id: life.foothold,
            state_timer: 0.0,
            idle_duration,
            chase_timeout: 10.0,
            detection_timer: 0.0,
            speed_variation_timer: 0.0,
            aggro: false,
            aggro_range,
            max_leash_distance,
            last_player_x: 0.0,
            last_player_y: 0.0,
            jump_velocity: -350.0,
            on_ground: true,
            hesitation_timer: 0.0,
            speed_multiplier: 1.0,
        }
    }
    
    /// Mark this mob as aggroed by the player (called when hit by weapon)
    pub fn set_aggro(&mut self) {
        self.aggro = true;
        if self.state == MobStateType::Idle || self.state == MobStateType::Patrol {
            self.state = MobStateType::Chase;
            self.state_timer = 0.0;
        }
    }
    
    /// Calculate distance from spawn point
    pub fn distance_from_spawn(&self) -> f32 {
        let dx = self.x - self.spawn_x;
        let dy = self.y - self.spawn_y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Mob AI system
pub struct MobAI;

impl MobAI {
    /// Update all mobs in the map
    pub fn update_mobs(mob_states: &mut [MobState], map: &MapData, player_x: f32, player_y: f32, dt: f32) {
        for mob_state in mob_states.iter_mut() {
            mob_state.last_player_x = player_x;
            mob_state.last_player_y = player_y;
            Self::update_single_mob(mob_state, map, dt);
        }
    }
    
    /// Expose a helper for combat code: aggro a mob by its life index
    pub fn aggro_mob_by_life_index(mob_states: &mut [MobState], life_index: usize) {
        if let Some(mob_state) = mob_states.iter_mut().find(|ms| ms.life_index == life_index) {
            mob_state.set_aggro();
        }
    }
    
    /// Update a single mob
    fn update_single_mob(mob_state: &mut MobState, map: &MapData, dt: f32) {
        mob_state.state_timer += dt;
        mob_state.detection_timer += dt;
        mob_state.speed_variation_timer += dt;
        
        // Update speed variation every 2-3 seconds
        if mob_state.speed_variation_timer >= 2.5 {
            mob_state.speed_multiplier = 0.9 + (mob_state.x as f32 % 20.0) / 100.0; // Pseudo-random 0.9-1.1
            mob_state.speed_variation_timer = 0.0;
        }
        
        // Detection check every 0.5 seconds
        if mob_state.detection_timer >= 0.5 {
            Self::check_player_detection(mob_state, map);
            mob_state.detection_timer = 0.0;
        }
        
        // Leash check
        if mob_state.distance_from_spawn() > mob_state.max_leash_distance {
            Self::return_to_spawn(mob_state);
        }
        
        // State machine
        match mob_state.state {
            MobStateType::Idle => Self::handle_idle(mob_state, dt),
            MobStateType::Patrol => Self::handle_patrol(mob_state, map, dt),
            MobStateType::Chase => Self::handle_chase(mob_state, map, dt),
            MobStateType::Jump => Self::handle_jump(mob_state, map, dt),
            MobStateType::Attack => Self::handle_attack(mob_state, dt),
            MobStateType::KnockedBack => Self::handle_knocked_back(mob_state, map, dt),
        }
        
        // Apply physics
        Self::apply_physics(mob_state, map, dt);
    }
    
    /// Check if player is in detection range
    fn check_player_detection(mob_state: &mut MobState, map: &MapData) {
        if mob_state.aggro {
            return; // Already aggroed
        }
        
        let dx = mob_state.last_player_x - mob_state.x;
        let dy = mob_state.last_player_y - mob_state.y;
        let distance = (dx * dx + dy * dy).sqrt();
        
        // Check horizontal distance
        if distance <= mob_state.aggro_range {
            // Check vertical proximity (within 2 platform heights)
            let y_diff = dy.abs();
            if y_diff <= 200.0 {
                // Simple line-of-sight: check if there's a clear path
                // For now, just check if on same platform or nearby
                if let Some(fh) = map.footholds.iter().find(|f| f.id == mob_state.current_foothold_id) {
                    let player_on_platform = mob_state.last_player_x >= fh.x1.min(fh.x2) as f32
                        && mob_state.last_player_x <= fh.x1.max(fh.x2) as f32;
                    
                    if player_on_platform || y_diff < 100.0 {
                        mob_state.aggro = true;
                        mob_state.state = MobStateType::Chase;
                        mob_state.state_timer = 0.0;
                    }
                }
            }
        }
    }
    
    /// Return to spawn when leashed
    fn return_to_spawn(mob_state: &mut MobState) {
        if mob_state.state == MobStateType::Chase {
            mob_state.state = MobStateType::Patrol;
            mob_state.aggro = false;
            mob_state.state_timer = 0.0;
        }
        
        // Move toward spawn
        let dx = mob_state.spawn_x - mob_state.x;
        if dx.abs() > 5.0 {
            mob_state.direction = if dx > 0.0 { 1.0 } else { -1.0 };
            mob_state.velocity_x = mob_state.direction * mob_state.chase_speed;
        } else {
            mob_state.velocity_x = 0.0;
            mob_state.state = MobStateType::Idle;
            mob_state.state_timer = 0.0;
        }
    }
    
    /// Handle IDLE state
    fn handle_idle(mob_state: &mut MobState, _dt: f32) {
        mob_state.velocity_x = 0.0;
        
        // Randomly face different direction
        if mob_state.state_timer > mob_state.idle_duration * 0.5 {
            // Small chance to change direction
            if (mob_state.x as i32 % 100) < 5 {
                mob_state.direction *= -1.0;
            }
        }
        
        // Transition to PATROL after idle duration
        if mob_state.state_timer >= mob_state.idle_duration {
            mob_state.state = MobStateType::Patrol;
            mob_state.state_timer = 0.0;
            // Randomize next idle duration
            mob_state.idle_duration = 1.0 + (mob_state.x as f32 % 200.0) / 100.0;
        }
    }
    
    /// Handle PATROL state
    fn handle_patrol(mob_state: &mut MobState, map: &MapData, dt: f32) {
        // Update velocity
        mob_state.velocity_x = mob_state.direction * mob_state.current_speed * mob_state.speed_multiplier;
        
        // Find current foothold
        let foothold = map.footholds.iter().find(|fh| fh.id == mob_state.current_foothold_id);
        
        if let Some(fh) = foothold {
            let fh_left = fh.x1.min(fh.x2) as f32;
            let fh_right = fh.x1.max(fh.x2) as f32;
            
            // Check if at edge
            let at_left_edge = mob_state.x <= fh_left + 10.0;
            let at_right_edge = mob_state.x >= fh_right - 10.0;
            
            if at_left_edge || at_right_edge {
                // Hesitation before turning/jumping
                if mob_state.hesitation_timer < 0.4 {
                    mob_state.hesitation_timer += dt;
                    mob_state.velocity_x = 0.0;
                    return;
                }
                
                mob_state.hesitation_timer = 0.0;
                
                // 80% chance to turn, 20% chance to jump if connected platform exists
                let should_jump = (mob_state.x as i32 % 10) < 2; // 20% chance
                
                if should_jump {
                    // Try to jump to connected platform
                    if at_left_edge && fh.prev != 0 {
                        if map.footholds.iter().any(|f| f.id == fh.prev) {
                            // Jump to previous foothold
                            mob_state.state = MobStateType::Jump;
                            mob_state.velocity_y = mob_state.jump_velocity;
                            mob_state.on_ground = false;
                            mob_state.state_timer = 0.0;
                            return;
                        }
                    } else if at_right_edge && fh.next != 0 {
                        if map.footholds.iter().any(|f| f.id == fh.next) {
                            // Jump to next foothold
                            mob_state.state = MobStateType::Jump;
                            mob_state.velocity_y = mob_state.jump_velocity;
                            mob_state.on_ground = false;
                            mob_state.state_timer = 0.0;
                            return;
                        }
                    }
                }
                
                // Turn around
                mob_state.direction *= -1.0;
                mob_state.velocity_x = 0.0;
            }
            
            // Random direction change every 3-7 seconds
            if mob_state.state_timer > 3.0 && (mob_state.x as i32 % 200) < 5 {
                mob_state.direction *= -1.0;
                mob_state.state_timer = 0.0;
            }
        }
    }
    
    /// Handle CHASE state
    fn handle_chase(mob_state: &mut MobState, map: &MapData, dt: f32) {
        let dx = mob_state.last_player_x - mob_state.x;
        let dy = mob_state.last_player_y - mob_state.y;
        let distance = (dx * dx + dy * dy).sqrt();
        
        // Give up chase if too far or timeout
        if distance > mob_state.aggro_range * 1.5 || mob_state.state_timer > mob_state.chase_timeout {
            mob_state.state = MobStateType::Patrol;
            mob_state.aggro = false;
            mob_state.state_timer = 0.0;
            return;
        }
        
        // Check attack range (50 pixels)
        if distance <= 50.0 {
            mob_state.state = MobStateType::Attack;
            mob_state.state_timer = 0.0;
            mob_state.velocity_x = 0.0;
            return;
        }
        
        // Move toward player
        if dx.abs() > 5.0 {
            mob_state.direction = if dx > 0.0 { 1.0 } else { -1.0 };
            mob_state.velocity_x = mob_state.direction * mob_state.chase_speed * mob_state.speed_multiplier;
        } else {
            mob_state.velocity_x = 0.0;
        }
        
        // Try to jump to reach player on different platform
        if dy.abs() > 50.0 && mob_state.on_ground {
            // Check if player is on platform above/below
            if let Some(fh) = map.footholds.iter().find(|fh| fh.id == mob_state.current_foothold_id) {
                // Simple check: if player Y is significantly different, try jumping
                if dy < -80.0 && (mob_state.x as i32 % 20) < 2 { // 10% chance to jump up
                    mob_state.state = MobStateType::Jump;
                    mob_state.velocity_y = mob_state.jump_velocity;
                    mob_state.on_ground = false;
                    mob_state.state_timer = 0.0;
                }
            }
        }
    }
    
    /// Handle JUMP state
    fn handle_jump(mob_state: &mut MobState, _map: &MapData, _dt: f32) {
        // Jump physics handled in apply_physics
        // Return to previous state when landing
        if mob_state.on_ground {
            if mob_state.aggro {
                mob_state.state = MobStateType::Chase;
            } else {
                mob_state.state = MobStateType::Patrol;
            }
            mob_state.state_timer = 0.0;
        }
    }
    
    /// Handle ATTACK state
    fn handle_attack(mob_state: &mut MobState, _dt: f32) {
        // Attack animation duration (0.5-1.5 seconds)
        if mob_state.state_timer >= 1.0 {
            // Return to chase
            mob_state.state = MobStateType::Chase;
            mob_state.state_timer = 0.0;
        }
    }
    
    /// Handle KNOCKED_BACK state
    fn handle_knocked_back(mob_state: &mut MobState, _map: &MapData, _dt: f32) {
        // Brief invulnerability period
        if mob_state.state_timer >= 0.4 {
            mob_state.state = MobStateType::Idle;
            mob_state.state_timer = 0.0;
        }
    }
    
    /// Apply physics (horizontal movement only, keep Y aligned to spawn/foothold)
    fn apply_physics(mob_state: &mut MobState, map: &MapData, dt: f32) {
        // Horizontal integration
        mob_state.x += mob_state.velocity_x * dt;

        // Keep Y aligned with spawn/foothold height so mobs match map spawn points.
        // We do not apply gravity here; vertical motion (jumping) can be added later
        // once map/platform parsing is fully robust.
        if let Some(fh) = map.footholds.iter().find(|fh| fh.id == mob_state.current_foothold_id) {
            let dx = fh.x2 - fh.x1;
            let dy = fh.y2 - fh.y1;
            let ix = mob_state.x as i32;

            let fh_y = if dx != 0 {
                (fh.y1 + ((ix - fh.x1) * dy) / dx) as f32
            } else {
                fh.y1 as f32
            };

            mob_state.y = fh_y - 30.0;
        } else {
            // Fallback: keep original spawn Y within map bounds
            mob_state.y = mob_state.spawn_y
                .max(map.info.vr_top as f32)
                .min(map.info.vr_bottom as f32);
        }

        mob_state.on_ground = true;
        mob_state.velocity_y = 0.0;
    }
}
