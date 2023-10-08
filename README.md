# Harborz

Harborz is a lightweight music player written in Rust, designed with a focus on supporting handheld devices such as
 Pinephone. It utilizes the following libraries to provide a seamless music playback experience:

- [libadwaita](https://gitlab.gnome.org/GNOME/libadwaita): A library that enhances GTK 4 applications to better adhere
 to GNOME's Human Interface Guidelines (HIG).
- [gtk4-rs](https://gtk-rs.org/gtk4-rs/git/book/installation.html): Rust bindings for GTK 4, providing a powerful
 toolkit for building graphical user interfaces.
- [gstreamer](https://github.com/GStreamer/gstreamer): An open-source multimedia framework for creating streaming media
 applications.

## Installation

### Using GitHub Release
To quickly get started with Harborz, you can download the latest release from the
 [GitHub release page](https://github.com/ravenblackdusk/harborz/releases). Simply choose the appropriate package for
 your operating system and follow the installation instructions provided.

### Building from Source
If you prefer to build Harborz from source, follow these steps:

1. Ensure that you have the necessary dependencies installed:
 - [gstreamer](https://gstreamer.freedesktop.org/documentation/installing/on-linux.html?gi-language=c): Refer to the
 official gstreamer documentation for installation instructions. Note that gstreamer may already be installed on your
 system.
 - [gtk4-rs](https://gtk-rs.org/gtk4-rs/git/book/installation.html): Follow the installation guide provided by gtk4-rs
 to set up the necessary dependencies.
 - [libadwaita](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html): Refer to the libadwaita documentation
 for installation instructions specific to your platform.
2. Clone the Harborz repository to your local machine.
3. Navigate to the project directory and build Harborz using Cargo: `cargo build --release`
4. Once the build process is complete, you will find the Harborz executable at `target/release/harborz`.

### Cross compiling from amd64 to aarch64
The process is very slow and the resulting binary is around 10MB larger, but it works.
1. make sure you have docker and docker-compose installed.
2. run `docker run --rm --privileged multiarch/qemu-user-static --reset -p yes` every once in a while to enable qemu
 aarch64 emulation.
3. run `docker-compose up`, you will find Harborz executable at `target/aarch64/release/harborz`.

### Adding a Desktop Icon
To add a desktop icon for Harborz, follow these steps:

1. Copy the `Harborz.desktop` file to the appropriate directory for your distribution. For example:
 `sudo cp Harborz.desktop /usr/share/applications/`
2. Open the copied `Harborz.desktop` file in a text editor.
3. Replace `<harborz git directory>` with the actual directory path where you cloned the Harborz repository.
4. Save the file.

### SQLite Database
Harborz utilizes an SQLite database file named `harborz.sqlite`. The executable will create and use this file for
 storing music-related information.

## Contact and Support
For any questions, feedback, or support, feel free to join our [Harborz Telegram channel](https://t.me/harborzplayer).

We appreciate your interest in Harborz and hope you enjoy using it for your music playback needs!
