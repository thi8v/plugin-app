echo "Building Rust plugin..."
cargo build --target wasm32-unknown-unknown

echo "Removing previous WASM component..."
rm ../plugin_ie.wasm

echo "Creating WASM component..."
wasm-tools component new --output ../plugin_ie.wasm ../target/wasm32-unknown-unknown/debug/plugin_ie.wasm

echo "Done!"
