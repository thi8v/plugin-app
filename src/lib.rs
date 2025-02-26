use core::str;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use host::plugin_app::core::types::Command;
use host::{PluginHost, PluginInfo};
use wasmtime::Engine;

pub mod cmds;
pub mod host;

#[derive(Debug, Clone)]
pub struct Cmd {
    usage: String,
    description: String,
}

impl Cmd {
    pub fn new(usage: impl ToString, description: impl ToString) -> Cmd {
        Cmd {
            usage: usage.to_string(),
            description: description.to_string(),
        }
    }
}

type BuiltinFn = fn(&mut ExecutionCtx, &str, Vec<&str>) -> Result<(), ()>;

#[derive(Debug, Clone)]
pub enum Runner {
    /// Built-in command.
    Builtin(BuiltinFn),
    Wasm {
        /// The plugin where the command is defined
        plugin: String,
    },
}

impl Runner {
    pub fn run(&self, ctx: &mut ExecutionCtx, cmd: &str, args: Vec<&str>) -> Result<(), ()> {
        match self {
            Runner::Builtin(func) => (func)(ctx, cmd, args),
            Runner::Wasm { plugin } => {
                // we can unwrap here because we know the plugin exists, and if
                // it doesn't it's a bug in this app.
                let mut host = ctx.hosts.get(plugin).unwrap().lock().unwrap();
                let args = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
                host.call_run_command(cmd, &args);
                Ok(())
            }
        }
    }
}

impl From<BuiltinFn> for Runner {
    fn from(value: BuiltinFn) -> Self {
        Runner::Builtin(value)
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionCtx {
    /// Maps a command name to its informations
    cmds: HashMap<String, Cmd>,
    /// Maps a plugin name to its informations
    plugins: HashMap<String, PluginInfo>,
    /// Maps a plugin name to its plugin host
    hosts: HashMap<String, Arc<Mutex<PluginHost>>>,
    /// Wasm engine
    engine: Engine,
    /// The commands to add after initialization of the plugin
    new_cmds: Option<(String, Vec<Command>)>,
    /// Is the shell running?
    running: bool,
}

impl ExecutionCtx {
    pub fn load_plugin(&mut self, path: PathBuf) {
        let mut host = PluginHost::new(self.engine.clone(), path);
        let info = host.call_init();

        if self.hosts.contains_key(&info.name) {
            println!("ERR: a plugin with the same name is already loaded")
        }

        self.hosts
            .insert(info.name.clone(), Arc::new(Mutex::new(host)));
        self.plugins.insert(info.name.clone(), info.clone());
        self.new_cmds = Some((info.name, info.commands));
    }
}

#[derive(Debug, Clone)]
pub struct Shell {
    /// Maps the command name to its runner
    runners: HashMap<String, Runner>,
    exec_ctx: ExecutionCtx,
}

impl Shell {
    pub fn new() -> Shell {
        let mut shell = Shell {
            runners: HashMap::new(),
            exec_ctx: ExecutionCtx {
                cmds: HashMap::new(),
                plugins: HashMap::new(),
                hosts: HashMap::new(),
                engine: Engine::default(),
                new_cmds: None,
                running: true,
            },
        };

        shell.define_cmd(
            "quit",
            Cmd::new("quit", "Quit the shell."),
            (|ctx, _, _| {
                ctx.running = false;
                Ok(())
            }) as BuiltinFn,
        );

        shell.define_cmd(
            "help",
            Cmd::new("help [cmd..]", "Print all commands to the screen or an helpful message if a command is passed as argument"),
            cmds::help_exec as BuiltinFn
        );

        shell.define_cmd(
            "list-plugins",
            Cmd::new("list-plugins", "Print all the plugins currently loaded"),
            cmds::list_plugin_exec as BuiltinFn,
        );

        shell.define_cmd(
            "load",
            Cmd::new("load <path>", "Loads a new plugin."),
            cmds::load_exec as BuiltinFn,
        );

        shell
    }

    pub fn run(&mut self) -> Result<()> {
        let mut input = String::new();

        while self.exec_ctx.running {
            input.clear();

            print!(">> ");
            stdout().flush()?;

            stdin().read_line(&mut input)?;

            // remove the last character, the newline it's useless.
            input.pop();

            let args = Shell::parse_cmd(&input);

            if args.len() == 0 {
                continue;
            }

            let Some(runner) = self.runners.get_mut(args[0]) else {
                eprintln!(
                    "ERR: unknown command {:?}, type \"help\" to see all commands.",
                    args[0]
                );
                continue;
            };

            match runner.run(&mut self.exec_ctx, args[0], args[1..].to_vec()) {
                Ok(()) => {}
                Err(()) => {
                    println!("ERROR")
                }
            }

            self.handle_new_cmds();
        }
        Ok(())
    }

    pub fn define_cmd(&mut self, cmd_name: impl ToString, cmd: Cmd, runner: impl Into<Runner>) {
        let name = cmd_name.to_string();

        if name.len() >= 16
            && !name.contains(char::is_whitespace)
            && name.contains(char::is_alphanumeric)
        {
            panic!("{name:?} is not a correct command name, it must be 16 charcters or shorter, doesn't contain whitesapces and is alphanumeric")
        }

        self.exec_ctx.cmds.insert(name.clone(), cmd);
        self.runners.insert(name.clone(), runner.into());
    }

    pub fn handle_new_cmds(&mut self) {
        if self.exec_ctx.new_cmds.is_none() {
            return;
        }

        let (plugin_name, commands) = self.exec_ctx.new_cmds.clone().unwrap();

        for command in commands {
            self.define_cmd(
                command.name,
                Cmd::new(command.usage, command.description),
                Runner::Wasm {
                    plugin: plugin_name.clone(),
                },
            );
        }
        self.exec_ctx.new_cmds = None;
    }

    pub fn parse_cmd(cmd: &str) -> Vec<&str> {
        cmd.split_whitespace().collect()
    }
}
