# MC Modpack Manager

Minecraft Modpack Manager (`mcmpmgr`) is a simple cli minecraft modpack manager.

## Goals
- Declarative modpack definition for any modloader with a simple `mcmodpack.toml` file.
- The hashes and exact versions of mods are stored in a `mcmodpack.lock` to ensure mod versions are pinned and unmodified.
- Easy export and import to/from modrinth and curseforge modpack formats.
- Easily search and add for any mod on modrinth (and maybe curseforge if its allowed?)
- Specifying side-only mods which can be filtered on modpack export and mod download.
- TODO: Figure out the rest of the goals...
