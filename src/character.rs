use serde::{Deserialize, Serialize};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::flags;

// In-memory storage that simulates IndexedDB (works on all platforms)
// On web, you would replace this with actual IndexedDB calls
static STORAGE: Mutex<Option<HashMap<String, Vec<u8>>>> = Mutex::new(None);

/// Character data that gets saved and loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub name: String,
    pub job: usize,
    pub level: u32,
    pub exp: u32,
    pub hp: u32,
    pub max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub str: u32,
    pub dex: u32,
    pub int: u32,
    pub luk: u32,
}

/// Storage key for the character list
const CHARACTER_LIST_KEY: &str = "character_list";

/// Prefix for individual character storage keys
const CHARACTER_KEY_PREFIX: &str = "character_";

/// Initialize storage
fn init_storage() {
    let mut storage = STORAGE.lock().unwrap();
    if storage.is_none() {
        *storage = Some(HashMap::new());
    }
}

/// Store data in memory (simulates IndexedDB on web)
fn store_data(key: &str, data: &[u8]) {
    init_storage();
    let mut storage = STORAGE.lock().unwrap();
    if let Some(ref mut map) = *storage {
        map.insert(key.to_string(), data.to_vec());
    }
}

/// Load data from memory (simulates IndexedDB on web)
fn load_data(key: &str) -> Option<Vec<u8>> {
    init_storage();
    let storage = STORAGE.lock().unwrap();
    storage.as_ref()?.get(key).cloned()
}

impl CharacterData {
    pub fn new(name: String, job: usize) -> Self {
        let (hp, mp, str, dex, int, luk) = if flags::START_WITH_MAX_STATS {
            (9999, 9999, 999, 999, 999, 999)
        } else {
            (50, 50, 4, 4, 4, 4)
        };

        Self {
            name,
            job,
            level: 200,
            exp: 0,
            hp,
            max_hp: hp,
            mp,
            max_mp: mp,
            str,
            dex,
            int,
            luk,
        }
    }

    /// Get storage key for a specific character
    fn get_character_key(name: &str) -> String {
        format!("{}{}", CHARACTER_KEY_PREFIX, name)
    }

    /// Save character data to storage (IndexedDB on web, file on native)
    pub fn save(&self) -> Result<(), String> {
        // Serialize character data
        let json = serde_json::to_string(self)
            .map_err(|e| format!("Failed to serialize character: {}", e))?;

        // Save individual character
        let character_key = Self::get_character_key(&self.name);
        store_data(&character_key, json.as_bytes());

        // Update character list
        let mut character_names = Self::load_character_list();
        if !character_names.contains(&self.name) {
            character_names.push(self.name.clone());
            Self::save_character_list(&character_names)?;
        }

        if flags::VERBOSE_CHARACTER_IO {
            info!("Character '{}' saved to storage (key: {})", self.name, character_key);
        }

        Ok(())
    }

    /// Load all characters from storage
    pub fn load_all() -> Vec<CharacterData> {
        let character_names = Self::load_character_list();
        let mut characters = Vec::new();

        for name in character_names {
            if let Some(character) = Self::load_one(&name) {
                characters.push(character);
            }
        }

        if flags::VERBOSE_CHARACTER_IO {
            info!("Loaded {} characters from storage", characters.len());
        }

        characters
    }

    /// Load a single character by name
    fn load_one(name: &str) -> Option<CharacterData> {
        let character_key = Self::get_character_key(name);

        if let Some(bytes) = load_data(&character_key) {
            if let Ok(json) = String::from_utf8(bytes) {
                if let Ok(character) = serde_json::from_str::<CharacterData>(&json) {
                    return Some(character);
                }
            }
        }

        if flags::VERBOSE_CHARACTER_IO {
            warn!("Failed to load character: {}", name);
        }
        None
    }

    /// Delete a character from storage
    pub fn delete(name: &str) -> Result<(), String> {
        // Delete individual character
        let character_key = Self::get_character_key(name);
        store_data(&character_key, &[]); // Clear the storage

        // Update character list
        let mut character_names = Self::load_character_list();
        character_names.retain(|n| n != name);
        Self::save_character_list(&character_names)?;

        if flags::VERBOSE_CHARACTER_IO {
            info!("Character '{}' deleted from storage", name);
        }

        Ok(())
    }

    /// Load the list of character names
    fn load_character_list() -> Vec<String> {
        if let Some(bytes) = load_data(CHARACTER_LIST_KEY) {
            if let Ok(json) = String::from_utf8(bytes) {
                if let Ok(names) = serde_json::from_str::<Vec<String>>(&json) {
                    return names;
                }
            }
        }
        Vec::new()
    }

    /// Save the list of character names
    fn save_character_list(names: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(names)
            .map_err(|e| format!("Failed to serialize character list: {}", e))?;

        store_data(CHARACTER_LIST_KEY, json.as_bytes());
        Ok(())
    }

    /// Create a test character (for debugging)
    pub fn create_test_character() -> Self {
        Self::new(flags::TEST_CHARACTER_NAME.to_string(), flags::TEST_CHARACTER_JOB)
    }
}
