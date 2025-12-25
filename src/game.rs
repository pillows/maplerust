use macroquad::prelude::*;
use crate::assets::{AssetManager, FrameData};
use crate::logo::{LogoAnimation, LogoState, display_logos, update_logo_animation, get_debug_text};
use crate::login;

const LOGO_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/00/UI/Logo.img";
const LOGO_CACHE_NAME: &str = "Logo.img";

/// Flag to skip logo animation for fast debugging
/// Set to true to skip directly to login screen
const SKIP_LOGOS: bool = true;

/// Load logo animation frames from WZ file
pub async fn load_logo_frames() -> (Vec<FrameData>, Vec<FrameData>) {
    info!("Loading logo animations...");
    let nexon = AssetManager::load_animation_frames_with_origins(LOGO_URL, LOGO_CACHE_NAME, "Nexon");
    let wizet = AssetManager::load_animation_frames_with_origins(LOGO_URL, LOGO_CACHE_NAME, "Wizet");
    // Note: While these run sequentially here, the WZ file will be cached after the first load,
    // making the second load much faster. For true parallel loading, you'd need the futures crate.
    let (nexon_frames, wizet_frames) = (nexon.await, wizet.await);
    info!("Loaded {} Nexon frames and {} Wizet frames", nexon_frames.len(), wizet_frames.len());
    
    // Validate frames are loaded
    if nexon_frames.is_empty() {
        error!("Nexon frames are empty!");
    }
    if wizet_frames.is_empty() {
        error!("Wizet frames are empty!");
    }
    
    (nexon_frames, wizet_frames)
}

/// Main game loop for logo animation
pub async fn run_logo_loop(nexon_frames: Vec<FrameData>, wizet_frames: Vec<FrameData>) {
    let mut animation = LogoAnimation::new();
    
    loop {
        clear_background(WHITE);
        let dt = get_frame_time();
        
        // Update logo animation
        update_logo_animation(&mut animation, dt, &wizet_frames, &nexon_frames);
        
        // Draw current animation frame
        display_logos(&animation.state, animation.current_frame, &wizet_frames, &nexon_frames);
        
        // Draw debug text
        let debug_text = get_debug_text(&animation, &wizet_frames, &nexon_frames);
        draw_text(&debug_text, 10.0, 60.0, 16.0, BLUE);
        
        // Transition to login when logos are completed
        if animation.state == LogoState::Completed {
            info!("Logo animation completed, transitioning to login screen");
            break;
        }
        
        next_frame().await
    }
}

/// Initialize and run the game
pub async fn run() {
    // Skip logos if flag is set for fast debugging
    if SKIP_LOGOS {
        info!("Skipping logo animation (SKIP_LOGOS flag enabled)");
        login::run_login_loop().await;
        return;
    }
    
    // Otherwise, load and display logos first
    let (nexon_frames, wizet_frames) = load_logo_frames().await;
    run_logo_loop(nexon_frames, wizet_frames).await;
    
    // After logos complete, transition to login
    login::run_login_loop().await;
}

