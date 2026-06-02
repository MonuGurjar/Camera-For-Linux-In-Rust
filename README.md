# Hyper Camera

A fast, lightweight webcam app for Linux inspired by the Xiaomi HyperOS camera UI. Built with Rust, GStreamer, and Slint.

I wanted a camera app that looks good on the Linux desktop but also handles V4L2 hardware switching properly without constantly locking up or freezing the camera feed.

## Features
- Clean, minimal UI with a frosted glass aesthetic.
- Dynamically probes your webcam so you only see resolutions and framerates that your hardware actually supports.
- Takes photos and records MP4 videos.
- Destroys and rebuilds the GStreamer pipeline when changing settings to prevent stubborn `EBUSY` kernel lockups.

## Dependencies

You'll need Rust and Cargo installed, plus GStreamer and curl. 

```sh
# Example for Ubuntu/Debian
sudo apt install curl libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly
```

*(Note: `curl` is needed if you still need to install Rust via rustup).*

## Installation

The easiest way to install and add it to your application launcher is to run our universal one-line installer. It will automatically detect your Linux distribution, install the required dependencies (Rust and GStreamer), compile the app, and set up a desktop shortcut.

```bash
curl -sSL https://raw.githubusercontent.com/MonuGurjar/Camera-For-Linux-In-Rust/main/install.sh | bash
```
*(Make sure to update the URL above once you upload this repository to GitHub).*

## Usage

Just search for "Hyper Camera" in your system launcher, or run `camera-app` from the terminal. 

*(Make sure `~/.local/bin` is in your system `$PATH`).*
