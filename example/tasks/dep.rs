fn main() {
    let now = wasi::clocks::wall_clock::now();
    println!("The unix time is now: {now:?}")
}
