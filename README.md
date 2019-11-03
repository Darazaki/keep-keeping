# Keep Keeping

Portable GUI and CLI applications to keep the latest files synchronized between
directories.

Those programs will replace the files with the oldest modification date with
the newest one without saving the old ones, use it at your own risk!

## Build & Install

In order to build `keep-keeping` you need to install [the Rust programming language](https://www.rust-lang.org/tools/install).

Then, open a terminal and enter the following commands:

```sh
git clone https://github.com/Darazaki/keep-keeping
cd keep-keeping
```

Next, follow the steps to build and install the CLI or the GUI.

### Build & Install the CLI

To build and install the CLI, type:

```sh
cd cli
cargo install --path .
```

If the installation succeeded, run `keep-keeping --help` to print help information.

### Build & Install the GUI

You first have to install a C compiler, CMake and Git because
the GUI will clone a version of [libui](https://github.com/andlabs/libui)
which has been made in C and uses CMake as its build system. (You
may also need a C++ compiler depending on your platform.)

To build the GUI, type:

```sh
cd gui
cargo build --release
```

If the build succeeded, the executable will be at
`../target/release/keep-keeping-gui` (or `../target/release/keep-keeping-gui.exe` on Windows).

You can then move it wherever you want and execute it to launch the application.

## Features and WIP

- [x] GUI;
- [x] CLI;
- [x] Synchronize two directories together;
- [ ] Unidirectional synchronization;
- [x] Allow initial path pointing to a file;
- [ ] Handle symbolic links:
    - recreate the link if it points to a path within the synchronized directory;
    - copy the contents if the link points to a path outside of the synchronized directory;
- [x] Handle macOS apps;
- [ ] Synchronize more than 2 directories at the same time;
- [ ] Check that the directories that are being synchronized are not parent and child.
