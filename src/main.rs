use macroquad::prelude::*;

mod assets;
use assets::AssetManager;

const PLAYER_SIZE: Vec2 = vec2(32.0, 32.0);
const PLAYER_SPEED: f32 = 200.0;
const JUMP_FORCE: f32 = 500.0;
const GRAVITY: f32 = 900.0; // gravity acceleration

struct Player {
    rect: Rect,
    velocity: Vec2,
    on_ground: bool,
    texture: Option<Texture2D>,
}

impl Player {
    fn new(x: f32, y: f32, texture: Option<Texture2D>) -> Self {
        Self {
            rect: Rect::new(x, y, PLAYER_SIZE.x, PLAYER_SIZE.y),
            velocity: Vec2::ZERO,
            on_ground: false,
            texture,
        }
    }

    fn update(&mut self, dt: f32, platforms: &[Rect]) {
        // Horizontal movement
        let mut direction = 0.0;
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            direction -= 1.0;
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            direction += 1.0;
        }
        self.velocity.x = direction * PLAYER_SPEED;

        // Apply gravity
        self.velocity.y += GRAVITY * dt;

        // Apply jump
        if self.on_ground
            && (is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::Up)
                || is_key_pressed(KeyCode::W))
        {
            self.velocity.y = -JUMP_FORCE;
            self.on_ground = false;
        }

        // Move X
        self.rect.x += self.velocity.x * dt;
        // Collision X
        for platform in platforms {
            if self.rect.overlaps(platform) {
                if self.velocity.x > 0.0 {
                    self.rect.x = platform.x - self.rect.w;
                } else if self.velocity.x < 0.0 {
                    self.rect.x = platform.x + platform.w;
                }
            }
        }

        // Move Y
        self.rect.y += self.velocity.y * dt;
        self.on_ground = false;
        // Collision Y
        for platform in platforms {
            if self.rect.overlaps(platform) {
                if self.velocity.y > 0.0 {
                    self.rect.y = platform.y - self.rect.h;
                    self.velocity.y = 0.0;
                    self.on_ground = true;
                } else if self.velocity.y < 0.0 {
                    self.rect.y = platform.y + platform.h;
                    self.velocity.y = 0.0;
                }
            }
        }

        // Screen bounds (simple floor if no platforms)
        if self.rect.y > screen_height() + 100.0 {
            // Reset if fell off
            self.rect.x = 100.0;
            self.rect.y = 100.0;
            self.velocity = Vec2::ZERO;
        }
    }

    fn draw(&self) {
        if let Some(tex) = &self.texture {
            draw_texture(tex, self.rect.x, self.rect.y, WHITE);
        } else {
            draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, BLUE);
        }
    }
}

struct SimpleButton {
    rect: Rect,
    label: String,
}

impl SimpleButton {
    fn new(x: f32, y: f32, w: f32, h: f32, label: &str) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            label: label.to_string(),
        }
    }

    fn is_clicked(&self) -> bool {
        let mouse_pos = mouse_position().into();
        if is_mouse_button_pressed(MouseButton::Left) && self.rect.contains(mouse_pos) {
            return true;
        }
        false
    }

    fn draw(&self) {
        draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, GRAY);
        let text_dims = measure_text(&self.label, None, 20, 1.0);
        draw_text(
            &self.label,
            self.rect.x + (self.rect.w - text_dims.width) / 2.0,
            self.rect.y + (self.rect.h + text_dims.height) / 2.0,
            20.0,
            BLACK,
        );
    }
}

#[macroquad::main("RustMaple")]
async fn main() {
    // Initial Asset Loading
    let mut status_message = "Loading Assets...".to_string();
    let necklace_texture = match AssetManager::load_texture(
        "https://res.cloudinary.com/dn2ie5quy/image/upload/v1593372205/Maplestory%20Monster%20buttons/necki_guaeee.png",
        "necki.png"
    ).await {
        Ok(tex) => {
            status_message = "Assets Loaded!".to_string();
            Some(tex)
        },
        Err(e) => {
            status_message = format!("Error: {}", e);
            None
        }
    };

    // Load PNG animation frames from WZ file
    let base_url = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/00/UI/Logo.img";
    let cache_name = "Logo.img";
    let base_path = "Nexon";

    // First, discover all available frame names
    info!("Discovering animation frames...");
    let frame_names = match AssetManager::get_wz_child_names(base_url, cache_name, base_path).await
    {
        Ok(names) => {
            // Filter to only PNG nodes (exclude origin, z, etc.)
            let mut png_frames: Vec<String> = names
                .into_iter()
                .filter(|name| {
                    // Try to parse as number to filter out non-numeric children
                    name.parse::<i32>().is_ok()
                })
                .collect();

            // Sort numerically for proper animation order
            png_frames.sort_by_key(|name| name.parse::<i32>().unwrap_or(0));

            info!(
                "Found {} animation frames: {:?}",
                png_frames.len(),
                png_frames
            );
            png_frames
        }
        Err(e) => {
            error!("Failed to discover frames: {}", e);
            Vec::new()
        }
    };

    // Now load all the frames
    let mut wz_animation_frames: Vec<Texture2D> = Vec::new();
    for frame_name in &frame_names {
        let frame_path = format!("{}/{}", base_path, frame_name);
        match AssetManager::load_wz_png_texture(base_url, cache_name, &frame_path).await {
            Ok(tex) => {
                info!("Loaded frame: {}", frame_path);
                wz_animation_frames.push(tex);
            }
            Err(e) => {
                error!("Failed to load frame {}: {}", frame_path, e);
            }
        }
    }

    if wz_animation_frames.is_empty() {
        error!("No animation frames loaded!");
    } else {
        info!(
            "Successfully loaded {} animation frames",
            wz_animation_frames.len()
        );
    }

    // Animation state
    let mut current_frame = 0;
    let mut frame_timer = 0.0;
    let frame_duration = 0.05; // 50ms per frame (20 FPS) - faster for more frames

    let mut player = Player::new(100.0, 100.0, necklace_texture);

    // Create some static platforms
    let platforms = vec![
        Rect::new(0.0, 500.0, 800.0, 50.0),   // Floor
        Rect::new(200.0, 400.0, 200.0, 30.0), // Platform 1
        Rect::new(500.0, 300.0, 200.0, 30.0), // Platform 2
    ];

    let reset_btn = SimpleButton::new(10.0, 10.0, 120.0, 40.0, "Reset Player");

    loop {
        clear_background(LIGHTGRAY);

        let dt = get_frame_time();

        // Update
        player.update(dt, &platforms);

        // Update animation frame
        if !wz_animation_frames.is_empty() {
            frame_timer += dt;
            if frame_timer >= frame_duration {
                frame_timer = 0.0;
                current_frame = (current_frame + 1) % wz_animation_frames.len();
            }
        }

        if reset_btn.is_clicked() {
            player.rect.x = 100.0;
            player.rect.y = 100.0;
            player.velocity = Vec2::ZERO;
            info!("Player reset!");
        }

        // Draw
        for platform in &platforms {
            draw_rectangle(platform.x, platform.y, platform.w, platform.h, DARKGREEN);
        }

        player.draw();
        reset_btn.draw();

        // Draw current animation frame if loaded
        if !wz_animation_frames.is_empty() {
            let tex = &wz_animation_frames[current_frame];
            draw_texture(tex, 300.0, 250.0, WHITE);
        }

        draw_text(&status_message, 150.0, 30.0, 20.0, BLACK);
        draw_text(
            "Arrows/WASD to move, Drag red/image item",
            10.0,
            580.0,
            20.0,
            DARKGRAY,
        );

        next_frame().await
    }
}
