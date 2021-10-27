# Hashums
Hashums is a tool for conveniently computing and comparing file hashes.

## Features
- Compute the SHA-256 hash of all selected files (more hashes should be relatively easy to add in the future)
- Compute a combined hash of all selected files (equal to the hash of the concatenation of each individual hash after sorting files lexicographically by their path)
- Selecting folders and drives will select all files contained within them
- Adds `Hashums` option to the `Send to` submenu of the context menu
- Adds `Hashums` option to the context menu when right-clicking drives or the background of the current folder (`Send to` does not appear in the context menu in these cases)
- Drag-and-drop files, folders, and drives onto the executable (or a shortcut or symbolic link to it)
- Invoke through the command line, with each argument being the path of a file, folder, or drive

## Known Issues
- Selecting multiple drives at once in Explorer will open an instance of Hashums for each drive
- Selecting drives and folders at the same time in Explorer, then right-clicking one of the selected drives, will not show any context menu option for Hashums

## Beneficial Quirks
- Selecting drives and folders at the same time in Explorer, then right-clicking one of the selected folders, will enable the `Send to` submenu in the context menu, allowing multiple drives to be selected and sent to one instance of Hashums
