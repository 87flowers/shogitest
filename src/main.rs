#![feature(str_split_whitespace_remainder)]

mod engine;

fn main() -> std::io::Result<()> {
    let engine_path = std::env::args().nth(1).unwrap();
    let e = engine::EngineBuilder { path: engine_path }.init().unwrap();
    println!("Engine name: {}", e.name());
    Ok(())
}
