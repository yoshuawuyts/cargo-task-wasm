fn main() -> std::io::Result<()> {
    std::env::args().for_each(|arg| {
        println!("arg: {}", arg);
    });
    Ok(())
}
