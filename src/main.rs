use std::error::Error;

use plugin_app::Shell;

pub const WELCOME_MSG: &str = r#"Welcome to this app, in this app you can load and unload plugins at runtime.
Type "help" to get some help."#;

fn main() -> Result<(), Box<dyn Error>> {
    println!("{WELCOME_MSG}");
    let mut shell = Shell::new();
    shell.run()?;
    Ok(())
}
