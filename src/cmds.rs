use std::path::PathBuf;

use crate::ExecutionCtx;

pub fn help_exec(ctx: &mut ExecutionCtx, _: &str, args: Vec<&str>) -> Result<(), ()> {
    if args.len() != 0 {
        todo!("Add support for help messages of individual commands")
    }

    println!("All commands:");
    let mut cmds = ctx.cmds.iter().map(|(_, cmd)| cmd).collect::<Vec<_>>();
    cmds.sort_by(|a, b| a.usage.cmp(&b.usage));
    for cmd in cmds {
        println!(" {:16} - {}", cmd.usage, cmd.description);
    }
    Ok(())
}

pub fn list_plugin_exec(ctx: &mut ExecutionCtx, _: &str, _: Vec<&str>) -> Result<(), ()> {
    let plugins = &ctx.plugins;
    if plugins.is_empty() {
        println!("There is currently no plugins loaded!");
        return Ok(());
    }

    println!("All loaded plugins:");
    for (_, info) in plugins {
        println!("  {:16} - {}", info.name, info.description);
    }
    Ok(())
}

pub fn load_exec(ctx: &mut ExecutionCtx, _: &str, args: Vec<&str>) -> Result<(), ()> {
    let Some(path) = args.get(0).map(|s| PathBuf::from(s)) else {
        println!("ERR: you must give the path to a WASM file to load.");
        return Err(());
    };
    ctx.load_plugin(path);
    println!("Plugin loaded successfully!");
    Ok(())
}
