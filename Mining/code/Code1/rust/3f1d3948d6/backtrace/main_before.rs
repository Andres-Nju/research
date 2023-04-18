fn main() {
    if cfg!(windows) && cfg!(target_env = "gnu") && cfg!(target_pointer_width = "32") {
        return
    }

    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 && args[1] == "fail" {
        foo();
    } else if args.len() >= 2 && args[1] == "double-fail" {
        double();
    } else {
        runtest(&args[0]);
    }
}
