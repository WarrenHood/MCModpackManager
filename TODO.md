# TODOs

This is a rather unorganised list of TODOs just so I can somewhat keep track of my plans without forgetting things.

## Plans for version 0.3.0

### Important

- Add NeoForge support.
- [ ] Concurrent mod downloads to speed up profile installation/updates.

### Nice to haves

- [ ] Concurrent mod metadata downloads to speed up running the `update` command.
- [ ] Allow for overriding file types for file merge apply policies rather than relying on the file extension
- [ ] Support resource and shaderpack installs rather than just mods.
- [ ] Get mmm working properly on `wayland` with Nix.
- [ ] Get cross-compilation to Windows working with the mingw toolchain.
- [ ] Preserve formatting as much as possible when merging files


## Done in version 0.2.0

### Important

- [X] Canonicalize relative path args with `profile` commands
- [X] Add a "merge" apply policy that can merge contents of certain file types.
    Eg. toml, json, and anything else that is reasonable (key-value type things... with nesting)
    This should also be able to merge folders, while recursively applying "merge" logic to individual files.
    
    Ie. Say we have folders A and B, with a.json and b.json in A. And the install dir already has folder A with some contents in a.json, and a folder B with x.txt and y.txt.

    merge (with conflict overrides on a file content level) should result in an install dir with A and B, where a.json and b.json are in A, and a.json is the result of merging a.json into the installed a.json (overwriting any existing key's values with the modpack's values), and the original files in folder B untouched (x.json and y.json)

    merge (retaining original/modified values) merge should result in an install dir with A and B, where a.json and b.json are in A, and a.json is the result of merging a.json into the installed a.json (retaining the existing values from the file in the install dir), and the original files in folder B untouched (x.json and y.json)
- [X] Test the merge apply policies when I am not half asleep. (nevermind, I tested while half asleep and seems good to me)
- [X] Show package version somewhere in `mmm`
- [X] Save userdata after removing profiles with the remove command

### Nice to haves

- [ ] Preserve formatting as much as possible when merging files
- [ ] Allow for overriding file types for file merge apply policies rather than relying on the file extension
- [ ] Support resource and shaderpack installs rather than just mods.
- [ ] Concurrent mod downloads to speed up profile installation/updates.
- [ ] Concurrent mod metadata downloads to speed up running the `update` command.
- [ ] Get mmm working properly on `wayland` with Nix.
- [ ] Get cross-compilation to Windows working with the mingw toolchain.
