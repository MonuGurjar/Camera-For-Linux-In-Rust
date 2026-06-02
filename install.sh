#!/usr/bin/env bash

set -e

# Update this URL once you host your code on GitHub
REPO_URL="https://github.com/MonuGurjar/Camera-For-Linux-In-Rust.git"

echo "========================================="
echo " Installing Hyper Camera "
echo "========================================="
echo ""

# 1. Detect OS and install dependencies
install_deps() {
    local DISTRO=$1
    echo "Installing dependencies for $DISTRO..."
    case $DISTRO in
        ubuntu)
            sudo apt-get update
            sudo apt-get install -y curl build-essential libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly
            ;;
        arch)
            sudo pacman -Syu --noconfirm --needed curl base-devel gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly
            ;;
        fedora)
            sudo dnf install -y curl gcc-c++ gstreamer1-devel gstreamer1-plugins-base-devel gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugins-ugly-free
            ;;
        opensuse)
            sudo zypper install -y curl gcc-c++ gstreamer-devel gstreamer-plugins-base-devel gstreamer-plugins-good gstreamer-plugins-bad gstreamer-plugins-ugly
            ;;
        *)
            echo "Unsupported distribution. Please install GStreamer and build tools manually."
            ;;
    esac
}

if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO_ID=${ID}
    DISTRO_LIKE=${ID_LIKE:-$ID}
else
    DISTRO_ID="unknown"
    DISTRO_LIKE="unknown"
fi

echo "1. Checking system distribution..."
if [[ "$DISTRO_LIKE" == *"ubuntu"* || "$DISTRO_LIKE" == *"debian"* || "$DISTRO_ID" == "debian" ]]; then
    install_deps "ubuntu"
elif [[ "$DISTRO_LIKE" == *"arch"* || "$DISTRO_ID" == "arch" ]]; then
    install_deps "arch"
elif [[ "$DISTRO_LIKE" == *"fedora"* || "$DISTRO_ID" == "fedora" ]]; then
    install_deps "fedora"
elif [[ "$DISTRO_LIKE" == *"suse"* || "$DISTRO_ID" == "opensuse" ]]; then
    install_deps "opensuse"
else
    echo "Could not automatically detect your Linux distribution."
    echo "Please select your distribution family:"
    PS3="Enter number (1-5): "
    select dist in "Ubuntu/Debian/Mint" "Arch/Manjaro/Garuda" "Fedora" "openSUSE" "Other (Skip deps)"; do
        case $dist in
            "Ubuntu/Debian/Mint") install_deps "ubuntu"; break;;
            "Arch/Manjaro/Garuda") install_deps "arch"; break;;
            "Fedora") install_deps "fedora"; break;;
            "openSUSE") install_deps "opensuse"; break;;
            "Other (Skip deps)") break;;
            *) echo "Invalid option.";;
        esac
    done
fi

echo ""
echo "2. Checking for Rust / Cargo..."
if ! command -v cargo &> /dev/null; then
    echo "Rust is not installed. Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo "Rust is already installed."
fi

echo ""
echo "3. Getting source code..."
if [ -f "Cargo.toml" ] && grep -q "camera" Cargo.toml; then
    echo "Found source code in current directory."
    SRC_DIR=$(pwd)
else
    echo "Cloning repository..."
    TEMP_DIR=$(mktemp -d)
    git clone "$REPO_URL" "$TEMP_DIR"
    SRC_DIR="$TEMP_DIR"
fi

cd "$SRC_DIR"

echo ""
echo "4. Building Hyper Camera in release mode..."
source "$HOME/.cargo/env" || true
cargo build --release

echo ""
echo "5. Installing binary..."
mkdir -p ~/.local/bin
cp target/release/camera ~/.local/bin/camera-app
echo "   -> Copied to ~/.local/bin/camera-app"

echo ""
echo "6. Installing desktop entry..."
mkdir -p ~/.local/share/applications
# Fallback to create desktop file if we just cloned and it's missing
if [ ! -f "camera-app.desktop" ]; then
    cat <<EOF > camera-app.desktop
[Desktop Entry]
Version=1.0
Type=Application
Name=Hyper Camera
Comment=A modern, HyperOS inspired camera app for Linux built with Rust and Slint.
Exec=camera-app
Icon=camera-photo
Terminal=false
Categories=AudioVideo;Video;Photography;
EOF
fi
cp camera-app.desktop ~/.local/share/applications/
echo "   -> Copied to ~/.local/share/applications/camera-app.desktop"

echo ""
echo "7. Updating desktop database..."
update-desktop-database ~/.local/share/applications || true

echo ""
echo "========================================="
echo " Installation complete! "
echo "========================================="
echo "You can now launch 'Hyper Camera' from your application launcher."
echo "Note: Make sure ~/.local/bin is in your PATH if you want to run it from terminal."
