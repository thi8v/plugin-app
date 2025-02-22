//! very simple plugin.
//!
//! # Compiling this plugin
//! rustc --crate-type cdylib -C linker-plugin-lto -C opt-level=z -C link-arg=--export-table --target wasm32-unknown-unknown simple_plug.rs
//!
//! # Optimizing this plugin
//! wasm-opt -Oz --strip-debug -o simple_plug.wasm simple_plug.wasm

#[link(wasm_import_module = "plugin-app")]
extern "C" {
    fn print(ptr: *const u8, len: usize);
}

#[no_mangle]
extern "C" fn init() -> fn() {
    printw("Hello world from the plugin! I'm initializating :)");
    call_from_rust
}

fn call_from_rust() {
    printw("You call me from Rust but i was actually a wasm function DAYUMN");
}

pub fn printw(text: &str) {
    // SAFETY: it should be fine trust me :D
    unsafe {
        print(text.as_ptr(), text.len());
    }
}
