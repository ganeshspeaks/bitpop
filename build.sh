#!/bin/bash

echo "Building BitPop (Rust)..."
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed!"
    echo ""
    echo "Install Rust with:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Check for GTK4 development files
if ! pkg-config --exists gtk4; then
    echo "❌ GTK4 development files not found!"
    echo ""
    echo "Install with:"
    echo "  sudo apt install libgtk-4-dev build-essential"
    echo ""
    exit 1
fi

# Build release binary
echo "🔨 Building release binary..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo ""
echo "📦 Installing..."

# Create directories
mkdir -p ~/.local/bin
mkdir -p ~/.local/share/applications

# Copy binary
cp target/release/bitpop ~/.local/bin/

# Create desktop entry
cat > ~/.local/share/applications/bitpop.desktop << 'EOF'
[Desktop Entry]
Name=BitPop
Comment=Quick access popup for time, battery, and controls
Exec=bitpop
Icon=preferences-system
Terminal=false
Type=Application
Categories=Utility;
EOF

echo "✓ BitPop installed successfully!"
echo ""
echo "Usage:"
echo "  Run 'bitpop' from terminal or application launcher"
echo ""
echo "Keyboard Shortcuts:"
echo "  • ESC - Close the popup"
echo "  • Tab - Navigate between buttons"
echo "  • Enter - Activate focused button"
echo "  • Click outside - Auto-dismiss"
echo ""
echo "To bind to a keyboard shortcut in KDE:"
echo "  1. Open System Settings > Shortcuts > Custom Shortcuts"
echo "  2. Add new shortcut"
echo "  3. Trigger: Choose your key combo (e.g., Meta+B)"
echo "  4. Action: Command - bitpop"
