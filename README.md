# Keep Keeping

A portable GUI and CLI application to keep the latest files synchronized between directories.

This program will overwrite the files with the oldest modification date with the newest one without saving the old ones, use it at your own risk!

## Features and WIP

- [x] GUI;
- [x] CLI;
- [x] Synchronize two directories together;
- [ ] Unidirectional synchronization;
- [x] Allow initial path pointing to a file;
- [ ] Handle symbolic links:
    - recreate the link if it points to a path within the synchronized directory;
    - copy the contents if the link points to a path outside of the synchronized directory;
- [x] Handle macOS apps.

## Warranties

I do not provide any warranties with this application, use it at your own risk.