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

#[macroquad::main("RustMaple")]
async fn main() {
    game::run().await;
}
