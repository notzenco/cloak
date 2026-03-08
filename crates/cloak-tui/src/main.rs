use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: cloak-tui <image-path>");
        process::exit(1);
    }

    let data = fs::read(&args[1]).unwrap_or_else(|e| {
        eprintln!("error: failed to read {}: {e}", args[1]);
        process::exit(1);
    });

    if let Err(e) = cloak_tui::run_tui(&data, &args[1]) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
