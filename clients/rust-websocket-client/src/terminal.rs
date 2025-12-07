use std::io::{self, stdin, stdout, Write};

/// Read a line from stdin with a prompt
pub fn read_line(prompt: &str) -> io::Result<String> {
    print!("{}", prompt);
    stdout().flush()?;
    
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_string())
}

/// Display a message to stdout
pub fn display_message(message: &str) {
    println!("{}", message);
}

/// Display an error message to stderr
#[allow(dead_code)]
pub fn display_error(message: &str) {
    eprintln!("Error: {}", message);
}

/// Display a debug message
#[allow(dead_code)]
pub fn display_debug(message: &str) {
    #[cfg(debug_assertions)]
    println!("Debug: {}", message);
}
