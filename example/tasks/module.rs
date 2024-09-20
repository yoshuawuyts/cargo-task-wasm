//! Calls a function from a local `mod.rs` file

mod shared;

pub fn main() {
    println!("hello from the main module");
    shared::hello();
}
