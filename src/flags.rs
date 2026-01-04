/// Centralized debug and configuration flags for the game
/// Modify these values to control game behavior for debugging and testing

/// Skip logo animation and go directly to login screen
pub const SKIP_LOGOS: bool = true;

/// Skip login screen and go directly to character selection
pub const SKIP_LOGIN: bool = true;

/// Skip all screens and go directly to gameplay with first available character
pub const SKIP_TO_GAME: bool = true;

/// Enable movable mode by default in character creation
pub const DEFAULT_MOVABLE_MODE: bool = false;

/// Enable debug UI overlays (FPS, coordinates, etc.)
pub const SHOW_DEBUG_UI: bool = false;

/// Show FPS counter (separate from full debug UI)
pub const SHOW_FPS: bool = true;

/// Show debug map loader UI (press M to toggle)
pub const SHOW_MAP_LOADER: bool = true;

/// Enable verbose logging for asset loading
pub const VERBOSE_ASSET_LOADING: bool = false;

/// Enable verbose logging for character save/load operations
pub const VERBOSE_CHARACTER_IO: bool = false;

/// Use in-memory storage instead of persistent storage (for testing)
pub const USE_MEMORY_STORAGE: bool = false;

/// Auto-create a test character if no characters exist
pub const AUTO_CREATE_TEST_CHARACTER: bool = false;

/// Test character name (used if AUTO_CREATE_TEST_CHARACTER is true)
pub const TEST_CHARACTER_NAME: &str = "TestHero";

/// Test character job (used if AUTO_CREATE_TEST_CHARACTER is true)
pub const TEST_CHARACTER_JOB: usize = 0;

/// Enable character creation validation (enforce min/max name length, etc.)
pub const ENABLE_CHARACTER_VALIDATION: bool = true;

/// Minimum character name length
pub const MIN_CHARACTER_NAME_LENGTH: usize = 3;

/// Maximum character name length
pub const MAX_CHARACTER_NAME_LENGTH: usize = 12;

/// Maximum number of characters per account
pub const MAX_CHARACTERS: usize = 3;

/// Enable collision detection in game world
pub const ENABLE_COLLISION: bool = true;

/// Player movement speed multiplier for debugging
pub const PLAYER_SPEED_MULTIPLIER: f32 = 1.0;

/// Enable camera debug mode (free camera movement)
pub const CAMERA_DEBUG_MODE: bool = false;

/// Show hitboxes and collision boundaries
pub const SHOW_HITBOXES: bool = true;

/// Render portals (set to false to improve FPS if needed)
pub const RENDER_PORTALS: bool = true;

/// Enable god mode (invincibility)
pub const GOD_MODE: bool = false;

/// Start with max stats
pub const START_WITH_MAX_STATS: bool = false;

/// Enable verbose map loading logs
pub const VERBOSE_MAP_LOADING: bool = false;

impl DebugFlags {
    /// Check if we should skip directly to a specific state
    pub fn get_initial_game_state() -> InitialGameState {
        if SKIP_TO_GAME {
            InitialGameState::InGame
        } else if SKIP_LOGIN {
            InitialGameState::CharacterSelection
        } else if SKIP_LOGOS {
            InitialGameState::Login
        } else {
            InitialGameState::Logos
        }
    }

    /// Check if debug UI should be shown
    pub fn should_show_debug_ui() -> bool {
        SHOW_DEBUG_UI
    }

    /// Get player speed with debug multiplier applied
    pub fn get_player_speed(base_speed: f32) -> f32 {
        base_speed * PLAYER_SPEED_MULTIPLIER
    }

    /// Check if verbose logging is enabled for a specific category
    pub fn is_verbose_logging_enabled(category: LogCategory) -> bool {
        match category {
            LogCategory::AssetLoading => VERBOSE_ASSET_LOADING,
            LogCategory::CharacterIO => VERBOSE_CHARACTER_IO,
        }
    }
}

/// Helper struct for debug flags (empty, just used for namespace)
pub struct DebugFlags;

/// Categories for verbose logging
pub enum LogCategory {
    AssetLoading,
    CharacterIO,
}

/// Initial game state based on skip flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitialGameState {
    Logos,
    Login,
    CharacterSelection,
    InGame,
}
