use anyhow::Result;
use plugin_app::Shell;

pub const WELCOME_MSG: &str = r#"Welcome to this app, in this app you can load and unload plugins at runtime.
Type "help" to get some help."#;

fn main() -> Result<()> {
    println!("{WELCOME_MSG}");
    let mut shell = Shell::new();
    shell.run()?;

    // println!();
    // let engine = wasmtime::Engine::default();
    // let mut host = PluginHost::try_new(engine.clone(), "plugin_ie.wasm")?;
    // dbg!(host.call_init());
    // host.call_run_command("hello", &["french".to_string()]);
    Ok(())
}
