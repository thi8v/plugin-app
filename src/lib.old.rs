use core::str;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use host::{PluginHost, PluginInfo};
use wasmtime::*;

pub mod host;

pub fn parse_cmd(cmd: &str) -> Vec<&str> {
    cmd.split_whitespace().collect()
}

pub type NativeExec = fn(Shell, Vec<&str>) -> Result<(), ()>;

pub enum CmdImpl {
    Native(NativeExec),
    Wasm { plugin_id: PluginId, cmd: String },
}

impl CmdImpl {
    pub fn call(&self, shell: Shell, cmd: &str, args: Vec<&str>) -> Result<(), ()> {
        match self {
            Self::Native(func) => (func)(shell, args),
            Self::Wasm { plugin_id, cmd } => todo!("IMPLEMENT THE WASM CALL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginId(u32);

pub struct Plugin {
    pub info: PluginInfo,
    pub host: PluginHost,
}

pub struct ShellImpl {
    cmd_descs: HashMap<String, Cmd>,
    /// is the Shell currently running?
    running: bool,
    /// the id of the most recent plugin loaded
    last_id: PluginId,
    engine: Engine,
    plugin_ids: HashMap<String, PluginId>,
    plugins: HashMap<PluginId, Arc<Mutex<Plugin>>>,
    cmd_execs: HashMap<String, CmdImpl>,
}

impl Debug for ShellImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellCtx")
            .field("cmd_descs", &self.cmd_descs)
            .field("running", &self.running)
            .field("last_id", &self.last_id)
            .field("plugin_ids", &self.plugin_ids)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct Shell {
    inner: Arc<Mutex<ShellImpl>>,
}

impl Shell {
    pub fn new() -> Shell {
        let mut shell = Shell {
            inner: Arc::new(Mutex::new(ShellImpl {
                cmd_descs: HashMap::new(),
                running: true,
                last_id: PluginId(0),
                engine: Engine::default(),
                plugin_ids: HashMap::new(),
                plugins: HashMap::new(),
                cmd_execs: HashMap::new(),
            })),
        };

        shell.register_cmd(
            "quit",
            Cmd {
                usage: "quit".to_string(),
                description: "Quit the app".to_string(),
            },
            |shell, _| {
                shell.inner.lock().unwrap().running = false;
                // shell.running = false;
                Ok(())
            },
        );

        shell.register_cmd(
            "load",
            Cmd {
                usage: "load <wasm file>".to_string(),
                description: "Load, and initialize the plugin from the given file path".to_string(),
            },
            Cmd::load_exec,
        );

        shell.register_cmd(
            "help",
            Cmd {
                usage: "help".to_string(),
                description: "Show all commands".to_string(),
            },
            Cmd::help_exec,
        );

        shell.register_cmd(
            "plugins",
            Cmd {
                usage: "plugins".to_string(),
                description: "List all plugins loaded".to_string(),
            },
            Cmd::plugins_exec,
        );

        shell
    }

    pub fn register_cmd(&self, name: impl ToString, cmd: Cmd, exec: NativeExec) {
        let mut inner = self.inner.lock().unwrap();
        let name = name.to_string();

        if name.len() >= 16
            && !name.contains(char::is_whitespace)
            && name.contains(char::is_alphanumeric)
        {
            panic!("{name:?} is not a correct command name, it must be 16 charcters or shorter, doesn't contain whitesapces and is alphanumeric")
        }

        inner.cmd_execs.insert(name.clone(), CmdImpl::Native(exec));
        inner.cmd_descs.insert(name, cmd);
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let inner = self.inner.lock().unwrap();
        let mut input = String::new();

        while inner.running {
            input.clear();

            print!(">> ");
            stdout().flush()?;

            stdin().read_line(&mut input)?;

            // remove the last character, the newline it's useless.
            input.pop();

            let args = parse_cmd(&input);

            if args.len() == 0 {
                println!("ERR: unknown command {input:?}, type \"help\" to see all commands.");
                continue;
            }

            // TODO: change this let-else in a if xxx.is_some()
            let Some(_) = self.get_cmd(args[0]) else {
                println!("ERR: unknown command {input:?}, type \"help\" to see all commands.");
                continue;
            };
            let exec = inner.cmd_execs.get(args[0]).unwrap();

            drop(inner);
            match exec.call(self.clone(), args[0], args[1..].to_vec()) {
                Ok(()) => {}
                Err(_) => {
                    // println!("ERR: command encountered errors:\n{e:?}");
                    continue;
                }
            }
        }
        Ok(())
    }

    pub fn get_cmd(&self, name: &str) -> Option<Cmd> {
        let inner = self.inner.lock().unwrap();
        inner.cmd_descs.get(name).cloned()
    }

    pub fn load_plugin(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let mut inner = self.inner.lock().unwrap();

        let mut host = PluginHost::new(
            inner.engine.clone(),
            Arc::new(Mutex::new(self.clone())),
            path,
        );
        let info = host.call_init();

        inner.last_id.0 += 1;
        let id = inner.last_id.clone();

        assert!(!inner.plugin_ids.contains_key(&info.name));
        inner.plugin_ids.insert(info.name.clone(), id.clone());

        inner
            .plugins
            .insert(id, Arc::new(Mutex::new(Plugin { info, host })));

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Cmd {
    /// Usage of the command
    usage: String,
    // aliases: Vec<String>, // TODO: add later
    description: String,
}

impl Cmd {
    pub fn help_exec(shell: Shell, _: Vec<&str>) -> Result<(), ()> {
        let shell = shell.inner.lock().unwrap();

        println!("All commands:");
        let mut cmds = shell
            .cmd_descs
            .iter()
            .map(|(_, cmd)| cmd)
            .collect::<Vec<_>>();
        cmds.sort_by(|a, b| a.usage.cmp(&b.usage));
        for cmd in cmds {
            println!(" {:16} - {}", cmd.usage, cmd.description);
        }
        Ok(())
    }

    pub fn plugins_exec(shell: Shell, _: Vec<&str>) -> Result<(), ()> {
        let shell = shell.inner.lock().unwrap();

        let plugins = &shell.plugins;
        if plugins.is_empty() {
            println!("There is currently no plugins loaded!");
            return Ok(());
        }

        println!("All loaded plugins:");
        for (id, plugin) in &shell.plugins {
            let info = &plugin.lock().unwrap().info;

            println!(
                "  {:20} - {}",
                format!("{}({})", info.name, id.0),
                info.description
            );
        }
        Ok(())
    }

    pub fn load_exec(mut shell: Shell, args: Vec<&str>) -> Result<(), ()> {
        // let mut shell = shell.inner.lock().unwrap();

        let Some(path) = args.get(0).map(|s| PathBuf::from(s)) else {
            println!("ERR: you must give the path to a WASM file to load.");
            return Err(());
        };
        shell.load_plugin(path).unwrap();
        Ok(())
    }
}
