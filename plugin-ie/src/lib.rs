//! Simple plugin for demonstration only purposes.

// #[cfg(not(target_arch = "wasm32"))]
// compile_error!("This crate must be compiled for the wasm32 target!");

wit_bindgen::generate!(in "../wit/plugin.wit");
use plugin_app::core::host_app::{define_cmd, log, Level};

pub struct PluginIe;

impl Guest for PluginIe {
    fn init() -> PluginInfo {
        log(Level::Debug, "Hello my friend!");
        define_cmd(
            "hello",
            "hello <language>",
            "Says hello in the given language",
        );

        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    fn run_command(name: String, args: Vec<String>) {
        match name.as_str() {
            "hello" => {
                if args.len() != 1 {
                    log(Level::Error, "hello command expects the language you want to say hello in as the first argument");
                    return;
                }
                match args[0].as_str() {
                    "english" => log(Level::Info, "Hello!"),
                    "french" => log(Level::Info, "Bonjour!"),
                    "italian" => log(Level::Info, "Ciao!"),
                    "german" => log(Level::Info, "Hallo!"),
                    lang => log(Level::Warn, &format!("unsupported language {lang}")),
                }
            }
            _ => {
                log(Level::Error, "command not defined in this plugin");
                return;
            }
        }
    }
}

export!(PluginIe);
