# TODOs

This is a rather unorganised list of TODOs just so I can somewhat keep track of my plans without forgetting things.

## Plans for version 0.2.0

### Important

- [ ] Canonicalize relative path args with `profile` commands
- [ ] Add a "merge" apply policy that can merge contents of certain file types.
    Eg. toml, json, and anything else that is reasonable (key-value type things... with nesting)
    This should also be able to merge folders, while recursively applying "merge" logic to individual files.
    
    Ie. Say we have folders A and B, with a.json and b.json in A. And the install dir already has folder A with some contents in a.json, and a folder B with x.txt and y.txt.

    merge (with conflict overrides on a file content level) should result in an install dir with A and B, where a.json and b.json are in A, and a.json is the result of merging a.json into the installed a.json (overwriting any existing key's values with the modpack's values), and the original files in folder B untouched (x.json and y.json)

    merge (retaining original/modified values) merge should result in an install dir with A and B, where a.json and b.json are in A, and a.json is the result of merging a.json into the installed a.json (retaining the existing values from the file in the install dir), and the original files in folder B untouched (x.json and y.json)

### Nice to haves

- [ ] Support resource and shaderpack installs rather than just mods.
- [ ] Concurrent mod downloads to speed up profile installation/updates.
- [ ] Concurrent mod metadata downloads to speed up running the `update` command.
- [ ] Get mmm working properly on `wayland` with Nix.
