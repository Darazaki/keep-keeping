# Keep Keeping

Portable GUI and CLI applications to keep the latest files synchronized between
directories.

Those programs will replace the files with the oldest modification date with
the newest one without saving the old ones, use it at your own risk!

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
