use core::str;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use component::TypedFunc;
use wasmtime::*;

pub fn parse_cmd(cmd: &str) -> Vec<&str> {
    cmd.split_whitespace().collect()
}

pub trait ExecFn<'a>: Fn(&mut ShellCtx, Vec<&str>) -> Result<(), ()> + 'a {}
// generic implementation to use it with ease
impl<'a, F> ExecFn<'a> for F where F: Fn(&mut ShellCtx, Vec<&str>) -> Result<(), ()> + 'a {}

pub type BoxedExec<'a> = Box<dyn ExecFn<'a>>;

pub struct WasmEnv<T> {
    engine: Engine,
    linker: Linker<T>,
    plugins: HashMap<PluginId, Plugin>,
}

impl<T> WasmEnv<T> {
    pub fn new(data: T) -> WasmEnv<T> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        linker
            .func_wrap(
                "plugin-app",
                "print",
                |mut caller: Caller<'_, T>, ptr: i32, len: i32| {
                    let mem = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .expect("Memory not found!");
                    let data = mem.data(&caller);
                    let start = ptr as usize;
                    let end = start + len as usize;
                    let msg = String::from_utf8_lossy(&data[start..end]);
                    println!("{msg}");
                },
            )
            .unwrap();

        WasmEnv {
            engine,
            linker,
            plugins: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PluginId(u32);

#[derive(Debug)]
pub struct Plugin {
    /// Name of the plugin, it must be unique to each plugin.
    name: String,
    /// Description of the plugin
    desc: String,
    /// Version of the plugin
    version: String,
    store: Store<()>,
    instance: Instance,
    module: Module,
}

#[derive(Clone)]
pub struct ShellCtx {
    cmd_descs: HashMap<String, Cmd>,
    running: bool,
    /// the id of the most recent plugin loaded
    last_id: PluginId,
    plugin_ids: HashMap<String, PluginId>,
    wasm: Arc<Mutex<WasmEnv<()>>>,
}

impl ShellCtx {
    pub fn load_plugin(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let _wasm = self.wasm.clone();
        let mut wasm = _wasm.lock().unwrap();

        let module = Module::from_file(&wasm.engine, path)?;

        let mut store = Store::new(&wasm.engine, ());

        let instance = wasm.linker.instantiate(&mut store, &module)?;

        for export in instance.exports(&mut store) {
            println!("{}: {:?}", export.name(), export.into_extern());
        }
        println!();
        let table = instance
            .get_table(&mut store, "__indirect_function_table")
            .expect("Function table not found");

        let init_fn = instance.get_typed_func::<(), i32>(&mut store, "init")?;

        let res = init_fn.call(&mut store, ())?;
        let func = table.get(&mut store, res as u64).unwrap();
        let res2 = func
            .unwrap_func()
            .unwrap()
            .typed::<(), ()>(&mut store)
            .unwrap();
        res2.call(&mut store, ())?;

        let plugin = Plugin {
            // TODO: initialize the name, desc and version of the plugin
            name: String::new(),
            desc: String::new(),
            version: String::new(),
            store,
            instance,
            module,
        };

        self.last_id.0 += 1;
        let id = self.last_id.clone();

        assert!(!self.plugin_ids.contains_key(&plugin.name));
        self.plugin_ids.insert(plugin.name.clone(), id.clone());

        wasm.plugins.insert(id, plugin);

        Ok(())
    }
}

impl Debug for ShellCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellCtx")
            .field("cmd_descs", &self.cmd_descs)
            .field("running", &self.running)
            .finish_non_exhaustive()
    }
}

pub struct Shell<'a> {
    cmd_execs: HashMap<String, BoxedExec<'a>>,
    ctx: ShellCtx,
}

impl<'a> Shell<'a> {
    pub fn new() -> Shell<'a> {
        let mut shell = Shell {
            cmd_execs: HashMap::new(),
            ctx: ShellCtx {
                cmd_descs: HashMap::new(),
                running: true,
                last_id: PluginId(0),
                plugin_ids: HashMap::new(),
                wasm: Arc::new(Mutex::new(WasmEnv::new(()))),
            },
        };

        shell.register_cmd(
            "quit",
            Cmd {
                usage: "quit".to_string(),
                description: "Quit the app".to_string(),
            },
            |ctx, _| {
                ctx.running = false;
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

    pub fn register_cmd(&mut self, name: impl ToString, cmd: Cmd, exec: impl ExecFn<'a>) {
        let name = name.to_string();
        if name.len() > 16 {
            todo!("MAYBE TOO LONG ?? IDK")
        }
        self.cmd_execs.insert(name.clone(), Box::new(exec));
        self.ctx.cmd_descs.insert(name, cmd);
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut input = String::new();

        while self.ctx.running {
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
            let Some(_) = self.get_cmd(args[0]).cloned() else {
                println!("ERR: unknown command {input:?}, type \"help\" to see all commands.");
                continue;
            };
            let exec = self.cmd_execs.get(args[0]).unwrap();

            match (exec)(&mut self.ctx, args[1..].to_vec()) {
                Ok(()) => {}
                Err(_) => {
                    // println!("ERR: command encountered errors:\n{e:?}");
                    continue;
                }
            }
        }
        Ok(())
    }

    pub fn get_cmd(&self, name: &str) -> Option<&Cmd> {
        self.ctx.cmd_descs.get(name)
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
    pub fn help_exec(ctx: &mut ShellCtx, _: Vec<&str>) -> Result<(), ()> {
        println!("All commands:");
        let mut cmds = ctx.cmd_descs.iter().map(|(_, cmd)| cmd).collect::<Vec<_>>();
        cmds.sort_by(|a, b| a.usage.cmp(&b.usage));
        for cmd in cmds {
            println!(" {:16} - {}", cmd.usage, cmd.description);
        }
        Ok(())
    }

    pub fn plugins_exec(ctx: &mut ShellCtx, _: Vec<&str>) -> Result<(), ()> {
        let _wasm = ctx.wasm.clone();
        let wasm = _wasm.lock().unwrap();
        let mut cmds = wasm.plugins.iter().collect::<Vec<_>>();
        if cmds.is_empty() {
            println!("There is currently no plugin loaded!");
            return Ok(());
        }
        println!("All loaded plugins:");
        cmds.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        for (id, plugin) in cmds {
            println!(
                " {:16} - {}",
                format!("{}({})", plugin.name, id.0),
                plugin.desc
            );
        }
        Ok(())
    }

    pub fn load_exec(ctx: &mut ShellCtx, args: Vec<&str>) -> Result<(), ()> {
        let Some(path) = args.get(0).map(|s| PathBuf::from(s)) else {
            println!("ERR: you must give the path to a WASM file to load.");
            return Err(());
        };
        ctx.load_plugin(path).unwrap();
        Ok(())
    }
}
