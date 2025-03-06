use engine::Engine;

mod engine;
mod eval;
mod search;

fn main() {
    let mut engine = Engine::new();
    engine.run();
}
