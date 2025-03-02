use engine::Engine;

mod engine;
mod search;
mod eval;

fn main() {
    let mut engine = Engine::new();
    engine.run();
}
