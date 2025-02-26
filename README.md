# Plugin App

This is an example of an application that extends its functionalities with a
plugin system. This project is written in Rust, plugins are compiled to WASM
components. They are then executed at runtime by the app.

# Software Requirements

To run this example you need various softwares:
- Rust (no way?) and Cargo
- [Wasm-tools][https://github.com/bytecodealliance/wasm-tools/]

# How to use it

1. `$ cd plugin-ie`
2. `$ ./compile.sh`
3. `$ cd ..`
4. `$ cargo run`
5. `>> load plugin_ie.wasm`
6. Use the project idk
