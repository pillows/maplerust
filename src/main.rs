mod assets;
mod game;
mod logo;
mod login;
mod character_selection;
mod character_creation;
mod character;
mod game_world;
mod flags;
mod map;
mod audio;
mod cursor;
mod character_info_ui;
mod minimap;
mod ui_windows;
mod cash_shop;
mod key_config;
mod chat_balloon;
mod game_menu;

#[macroquad::main("RustMaple")]
async fn main() {
    game::run().await;
}
