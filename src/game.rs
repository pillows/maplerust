use macroquad::prelude::*;
use crate::assets::{AssetManager, FrameData};
use crate::logo::{LogoAnimation, LogoState, display_logos, update_logo_animation, get_debug_text};
use crate::login;
use crate::character_selection;
use crate::character_creation;
use crate::game_world::GameplayState;
use crate::character::CharacterData;
use crate::flags::{self, DebugFlags, InitialGameState};

const LOGO_URL: &str = "https://scribbles-public.s3.us-east-1.amazonaws.com/tutorial/00/UI/Logo.img";
const LOGO_CACHE_NAME: &str = "/00/UI/Logo.img";

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

/// Game states
#[derive(PartialEq, Clone, Copy)]
enum GameState {
    Logos,
    Login,
    CharacterSelection,
    CharacterCreation,
    InGame,
}

/// Initialize and run the game
pub async fn run() {
    // Determine initial game state based on flags
    let initial_state = DebugFlags::get_initial_game_state();
    let mut game_state = match initial_state {
        InitialGameState::Logos => {
            info!("Starting from logo animation");
            GameState::Logos
        }
        InitialGameState::Login => {
            info!("Skipping logos (SKIP_LOGOS flag enabled)");
            GameState::Login
        }
        InitialGameState::CharacterSelection => {
            info!("Skipping to character selection (SKIP_LOGIN flag enabled)");
            GameState::CharacterSelection
        }
        InitialGameState::InGame => {
            info!("Skipping to game (SKIP_TO_GAME flag enabled)");
            GameState::InGame
        }
    };

    // Load logo frames if needed
    let (nexon_frames, wizet_frames) = if game_state == GameState::Logos {
        load_logo_frames().await
    } else {
        (Vec::new(), Vec::new())
    };

    // Initialize login state
    let mut login_state = login::LoginState::new();
    let mut login_assets_loaded = false;

    // Initialize character selection state
    let mut char_select_state = character_selection::CharacterSelectionState::new();
    let mut char_select_assets_loaded = false;
    let mut char_select_needs_reload = false;

    // Initialize character creation state
    let mut char_create_state = character_creation::CharacterCreationState::new();
    let mut char_create_assets_loaded = false;

    // Initialize gameplay state (will be created when transitioning to game)
    let mut gameplay_state: Option<GameplayState> = None;
    let mut gameplay_assets_loaded = false;

    // Handle SKIP_TO_GAME flag - create test character or load first character
    if game_state == GameState::InGame {
        let character = if flags::AUTO_CREATE_TEST_CHARACTER {
            info!("Auto-creating test character");
            let test_char = CharacterData::create_test_character();
            let _ = test_char.save();
            test_char
        } else {
            let characters = CharacterData::load_all();
            if let Some(first_char) = characters.first() {
                info!("Loading first available character: {}", first_char.name);
                first_char.clone()
            } else {
                warn!("No characters found for SKIP_TO_GAME, creating test character");
                let test_char = CharacterData::create_test_character();
                let _ = test_char.save();
                test_char
            }
        };
        gameplay_state = Some(GameplayState::new(character));
    }

    // Handle AUTO_CREATE_TEST_CHARACTER for character selection
    if flags::AUTO_CREATE_TEST_CHARACTER && game_state == GameState::CharacterSelection {
        let characters = CharacterData::load_all();
        if characters.is_empty() {
            info!("Auto-creating test character for character selection");
            let test_char = CharacterData::create_test_character();
            let _ = test_char.save();
        }
    }

    // Main game loop
    loop {
        let dt = get_frame_time();

        match game_state {
            GameState::Logos => {
                // Run logo animation
                let mut animation = LogoAnimation::new();
                loop {
                    clear_background(WHITE);
                    let dt = get_frame_time();

                    update_logo_animation(&mut animation, dt, &wizet_frames, &nexon_frames);
                    display_logos(&animation.state, animation.current_frame, &wizet_frames, &nexon_frames);

                    let debug_text = get_debug_text(&animation, &wizet_frames, &nexon_frames);
                    draw_text(&debug_text, 10.0, 60.0, 16.0, BLUE);

                    if animation.state == LogoState::Completed {
                        info!("Logo animation completed, transitioning to login screen");
                        game_state = GameState::Login;
                        break;
                    }

                    next_frame().await;
                }
            }
            GameState::Login => {
                // Load login assets if not yet loaded
                if !login_assets_loaded {
                    login_state.load_assets().await;
                    login_assets_loaded = true;
                }

                // Run login screen
                login_state.update(dt);
                login_state.draw();

                // Check if should transition to character selection
                if login_state.should_transition_to_char_select() {
                    info!("Transitioning to character selection screen");
                    game_state = GameState::CharacterSelection;
                }

                next_frame().await;
            }
            GameState::CharacterSelection => {
                // Load character selection assets if not yet loaded
                if !char_select_assets_loaded {
                    char_select_state.load_assets().await;
                    char_select_assets_loaded = true;
                    char_select_needs_reload = true;
                }

                // Reload characters if needed (after coming back from character creation)
                if char_select_needs_reload {
                    char_select_state.reload_characters();
                    char_select_needs_reload = false;
                }

                // Run character selection screen
                char_select_state.update(dt);
                char_select_state.draw();

                // Check if should transition to character creation
                if char_select_state.should_transition_to_char_create() {
                    info!("Transitioning to character creation screen");
                    game_state = GameState::CharacterCreation;
                    // Reset character creation state for new character
                    char_create_state = character_creation::CharacterCreationState::new();
                    char_create_assets_loaded = false;
                }

                // Check if should transition to game
                if char_select_state.should_transition_to_game() {
                    if let Some(character) = char_select_state.get_selected_character() {
                        info!("Transitioning to game with character: {}", character.name);
                        gameplay_state = Some(GameplayState::new(character.clone()));
                        gameplay_assets_loaded = false;
                        game_state = GameState::InGame;
                    }
                }

                next_frame().await;
            }
            GameState::CharacterCreation => {
                // Load character creation assets if not yet loaded
                if !char_create_assets_loaded {
                    char_create_state.load_assets().await;
                    char_create_assets_loaded = true;
                }

                // Run character creation screen
                char_create_state.update(dt);
                char_create_state.draw();

                // Check if should transition back to character selection
                if char_create_state.should_transition_to_char_select() {
                    info!("Transitioning back to character selection");
                    game_state = GameState::CharacterSelection;
                    char_select_needs_reload = true; // Reload characters after creation
                }

                next_frame().await;
            }
            GameState::InGame => {
                // Create gameplay state if needed
                if gameplay_state.is_none() {
                    error!("Gameplay state is None in InGame state!");
                    game_state = GameState::CharacterSelection;
                    next_frame().await;
                    continue;
                }

                let state = gameplay_state.as_mut().unwrap();

                // Load gameplay assets if not yet loaded
                if !gameplay_assets_loaded {
                    state.load_assets().await;
                    gameplay_assets_loaded = true;
                }

                // Run gameplay
                state.update(dt);
                state.draw();

                next_frame().await;
            }
        }
    }
}

