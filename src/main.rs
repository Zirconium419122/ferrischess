use engine::Engine;

mod engine;
mod eval;
mod move_sorter;
mod piecesquaretable;
mod search;

fn main() {
    let mut engine = Engine::new();
    engine.run();
}
