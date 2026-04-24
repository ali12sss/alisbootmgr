#!/bin/bash
echo "Building Ali's Boot Manager..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "Installing binary to /usr/local/bin..."
    sudo cp target/release/rust-efi-manager /usr/local/bin/alisbootmgr
    
    echo "Installing menu shortcut..."
    sudo cp alisbootmgr.desktop /usr/share/applications/
    
    echo "Done! You can now find Ali's Boot Manager in your menu."
else
    echo "Build failed. Please ensure you have Rust installed."
fi
