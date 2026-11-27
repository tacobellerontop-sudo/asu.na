# AsuNa Launcher

One game manager for every launcher. AsuNa finds the games you have in **Steam, Epic Games, GOG Galaxy,
Battle.net, the EA app, Ubisoft Connect, Xbox / Microsoft Store and the Riot Client**, shows them in one
library, and lets you play, install and uninstall them from one place.

It's a native Rust app drawn on the GPU with [GPUI](https://www.gpui.rs) (the UI framework behind the Zed
editor) and the [gpui-component](https://github.com/longbridge/gpui-component) widget kit. There's no browser
engine inside, so it starts fast and stays small.

## What it does

- **One library** of installed games from every launcher, plus games you own but haven't installed (Steam and
  Epic), with box art, size on disk, play time and last played. Search, sort, favorites and hidden games.
- **Play** through the game's own launcher, or **run the game directly** without its launcher (per game).
- **Install and uninstall** through each launcher's own flow.
- **Get games:** search the Steam store plus the Battle.net and Riot catalogs and start an install.
- **Launchers page:** which launchers were found, where, and how many games each has; turn any off or get the
  ones you're missing.
- **Settings:** open on sign-in (minimized), start minimized, what to do when a game starts (stay open,
  minimize, or close AsuNa to free its memory), rescan on start, box art, online lookups, and launcher folders
  when auto-detection misses one.
- **Add your own games** (emulators, itch.io, DRM-free installs).
- **Custom title bar** with its own window controls.

### About running games without their launcher

"Run the game directly" starts the game's own `.exe`. Whether that works is up to the game:

- **Steam:** most games use Steam's DRM and will open Steam anyway. DRM-free Steam games start on their own;
  AsuNa sets `SteamAppId` so they don't bounce back through Steam.
- **EA, Ubisoft, Battle.net, Riot:** usually need their launcher for sign-in or anti-cheat.
- **Xbox / Game Pass:** protected; normally can't be started directly.
- **GOG and Epic** DRM-free titles generally work well.

## How each launcher is read

| Launcher | Installed games | Owned, not installed | Play / install / uninstall via |
| --- | --- | --- | --- |
| Steam (Win, macOS, Linux) | `libraryfolders.vdf` + `appmanifest_*.acf` | `userdata/*/config/localconfig.vdf`, names from the Steam store (cached) | `steam://rungameid`, `steam://install`, `steam://uninstall` |
| Epic Games (Win, macOS) | `Data/Manifests/*.item` | `catcache.bin` (best effort) | `com.epicgames.launcher://apps/…?action=launch / install` |
| GOG Galaxy (Win) | `HKLM\…\GOG.com\Games` | — | `GalaxyClient.exe /command=runGame`, `unins000.exe` |
| Battle.net (Win) | Blizzard uninstall entries | built-in catalog | `Battle.net.exe --exec="launch <code>"`, `battlenet://<code>` |
| EA app (Win) | uninstall entries + `__Installer\installerdata.xml` | — | `origin2://game/launch`, the game's uninstaller |
| Ubisoft Connect (Win) | `HKLM\…\Ubisoft\Launcher\Installs` | — | `uplay://launch / install / uninstall` |
| Xbox / Microsoft Store (Win) | `Get-AppxPackage` + `MicrosoftGame.config` | — | `shell:AppsFolder\<PFN>!<AppId>` |
| Riot Client (Win, macOS) | `RiotClientInstalls.json` + `Metadata/*.product_settings.yaml` | built-in catalog (all free) | `RiotClientServices --launch-product / --uninstall-product` |

Everything is read locally. The only network calls are to the Steam store (names of owned games, store search)
and for box art, which is cached on disk. Both can be turned off in Settings.

## Download

Grab the latest build from [Releases](https://github.com/tacobellerontop-sudo/asu.na/releases):

- **Windows:** `AsuNa.exe` (or the `.zip`). Unsigned for now, so SmartScreen may warn on first launch.
- **macOS (Apple silicon):** unzip and move *AsuNa Launcher.app* to Applications. Unsigned: right-click → Open
  the first time.
- **Linux:** `tar xzf` and run `asuna`. Needs a GPU with Vulkan.

## Build from source

```bash
cargo run -p asuna-app            # run the app
cargo test -p asuna-core          # launcher detection tests (no GUI needed, any OS)
cargo build --release -p asuna-app
```

Linux needs: `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-xcb-dev libfontconfig1-dev libvulkan-dev`.
The first build compiles GPUI and takes several minutes.

## Releases

`.github/workflows/build.yml` tests the core on Windows, macOS and Linux and builds the app for all three on
every push and pull request.

To publish a release, bump `version` under `[workspace.package]` in `Cargo.toml` and merge to `main`. When that
version has no tag yet, the workflow creates the `vX.Y.Z` tag and a GitHub Release with the Windows, macOS and
Linux builds attached. Versions with a hyphen (such as `0.3.0-beta.1`) are marked as pre-releases. Pushing a
`v*` tag by hand still works too.

## Layout

```
crates/
  asuna-core/          launcher detection, library, launching, settings (no GUI, fully tested)
    src/providers/     one file per launcher: detect() + scan()
    src/library.rs     runs providers in parallel, merges favorites / hidden / launch overrides
    src/actions.rs     opens launcher links or starts programs (allow-listed schemes only)
  asuna-app/           the GPUI app
    src/app.rs         state and behavior
    src/views.rs       drawing
```

### Adding a launcher

Add `crates/asuna-core/src/providers/<name>.rs` implementing `Provider`, register it in `providers/mod.rs`, add
its id to `LauncherId`, and add a test with fixture files.
