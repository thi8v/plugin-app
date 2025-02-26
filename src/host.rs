use std::{
    fmt::Display,
    path::Path,
    sync::{Arc, Mutex},
};

use wasmtime::{
    component::{bindgen, Component, Linker},
    Engine, Result, Store,
};

bindgen!(in "wit/plugin.wit");

use plugin_app::core::host_app::Level;

use crate::Shell;

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Debug => write!(f, "DEBUG"),
            Level::Info => write!(f, "INFO"),
            Level::Warn => write!(f, "WARN"),
            Level::Error => write!(f, "ERROR"),
        }
    }
}

struct PluginState {
    shell: Arc<Mutex<Shell>>,
}

impl plugin_app::core::host_app::Host for PluginState {
    fn log(&mut self, lvl: Level, msg: String) -> () {
        println!("{lvl}: {msg}")
    }

    fn define_cmd(&mut self, name: String, usage: String, description: String) {
        println!("NAME = {name:?}; USAGE = {usage:?}, DESCRIPTION = {description:?}")
    }
}

impl plugin_app::core::types::Host for PluginState {}

/// Anything needed to execute the WASM plugin.
#[allow(unused)] // TODO: this is temporary.
pub struct PluginHost {
    component: Component,
    linker: Linker<PluginState>,
    store: Store<PluginState>,
    bindings: Core,
}

impl PluginHost {
    pub fn try_new(
        engine: Engine,
        shell: Arc<Mutex<Shell>>,
        path: impl AsRef<Path>,
    ) -> Result<PluginHost> {
        let component = Component::from_file(&engine, path)?;

        let mut linker = Linker::new(&engine);
        Core::add_to_linker(&mut linker, |state: &mut PluginState| state)?;

        let mut store = Store::new(&engine, PluginState { shell });
        let bindings = Core::instantiate(&mut store, &component, &linker)?;

        // dbg!(bindings.call_init(&mut store)?);

        Ok(PluginHost {
            component,
            linker,
            store,
            bindings,
        })
    }

    #[track_caller]
    pub fn new(engine: Engine, shell: Arc<Mutex<Shell>>, path: impl AsRef<Path>) -> PluginHost {
        PluginHost::try_new(engine, shell, path).unwrap()
    }

    #[track_caller]
    pub fn call_init(&mut self) -> PluginInfo {
        self.bindings.call_init(&mut self.store).unwrap()
    }

    #[track_caller]
    pub fn call_run_command(&mut self, name: &str, args: &[String]) {
        self.bindings
            .call_run_command(&mut self.store, name, args)
            .unwrap()
    }
}
