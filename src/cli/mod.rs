pub fn parse_args() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 { args[1].clone() } else { "start".to_string() }
}
