use engine::Engine;

mod engine;
mod eval;
mod move_sorter;
mod piecesquaretable;
mod search;
mod time_management;
mod transposition_table;

fn main() {
    let mut engine = Engine::new();
    engine.run();
}
