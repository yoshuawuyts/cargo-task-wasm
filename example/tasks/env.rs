fn main() {
    let my_env = std::env::var("MY_ENV_VAR").unwrap();
    println!("MY_ENV_VAR={my_env}");
}
