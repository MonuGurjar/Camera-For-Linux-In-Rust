#!/bin/bash
set -e

echo "Checking and installing dependencies..."
if command -v apt-get &> /dev/null; then
    sudo apt-get update
    sudo apt-get install -y \
        libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
        gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
        gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
        gstreamer1.0-pipewire gstreamer1.0-libcamera \
        pkg-config
elif command -v dnf &> /dev/null; then
    sudo dnf install -y \
        gstreamer1-devel gstreamer1-plugins-base-devel \
        gstreamer1-plugins-good gstreamer1-plugins-bad-free \
        gstreamer1-plugins-ugly-free gstreamer1-plugin-pipewire \
        gstreamer1-plugin-libcamera pkgconfig
elif command -v pacman &> /dev/null; then
    sudo pacman -S --needed --noconfirm \
        gstreamer gst-plugins-base gst-plugins-good \
        gst-plugins-bad gst-plugins-ugly gst-plugin-pipewire \
        libcamera pkgconf
else
    echo "Unsupported package manager. Please install GStreamer, PipeWire, and libcamera manually."
fi
echo "Dependencies installed."
