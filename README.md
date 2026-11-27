# AsuNa Launcher

One game manager for every launcher. AsuNa finds the games you have in **Steam, Epic Games, GOG Galaxy,
Battle.net, the EA app, Ubisoft Connect and Xbox / Microsoft Store**, shows them in one library, and lets you
play, install and uninstall them from one place. Each launcher still does the real work (downloads, DRM,
cloud saves); AsuNa drives it through the launcher's own links and command line.

## What it does

- **One library.** Installed games from every launcher, plus games you own but have not installed
  (Steam and Epic), with box art, size on disk, play time and last played.
- **Play** any game with one click, through its own launcher.
- **Install** owned games: AsuNa opens the launcher's install flow for that exact game.
- **Get games.** Search the Steam store and the Battle.net catalog and start an install from the results.
- **Uninstall, open folder, open the store page,** favorite or hide any game.
- **Launchers page** shows which launchers were found, where, and how many games each has; turn any off,
  or get the ones you don't have yet.
- **Add your own games** (emulators, itch.io, DRM-free installs) by pointing at an executable.

## How each launcher is read

| Launcher | Finds installed games from | Finds owned, not installed | Play / install / uninstall via |
| --- | --- | --- | --- |
| Steam (Win, macOS, Linux) | `libraryfolders.vdf` + `appmanifest_*.acf` in every library | `userdata/*/config/localconfig.vdf`, names from the Steam store API (cached) | `steam://rungameid`, `steam://install`, `steam://uninstall` |
| Epic Games (Win, macOS) | `Data/Manifests/*.item` | `catcache.bin` catalog cache (best effort) | `com.epicgames.launcher://apps/…?action=launch / install` |
| GOG Galaxy (Win) | `HKLM\…\GOG.com\Games` registry | — | `GalaxyClient.exe /command=runGame`, `unins000.exe` |
| Battle.net (Win) | Uninstall registry entries from Blizzard | Built-in catalog of Blizzard titles | `Battle.net.exe --exec="launch <code>"`, `battlenet://<code>` |
| EA app (Win) | Uninstall entries + `__Installer\installerdata.xml` | — | `origin2://game/launch`, the game's uninstaller |
| Ubisoft Connect (Win) | `HKLM\…\Ubisoft\Launcher\Installs` | — | `uplay://launch`, `uplay://install`, `uplay://uninstall` |
| Xbox / Microsoft Store (Win) | `Get-AppxPackage` filtered to GDK games (`MicrosoftGame.config`) | — | `shell:AppsFolder\<PFN>!<AppId>`, Store page |

Everything is read locally. The only network calls are the Steam store API (names of owned games and store
search) and box art images.

## Develop

```bash
npm install
npm run dev        # Electron app with hot reload
npm run web        # UI only, in a browser, with sample data (no Electron needed)
npm test           # parser and provider tests (fixtures, no launchers needed)
npm run typecheck
npm run dist:win   # Windows installer in dist/
```

## Layout

```
src/
  shared/types.ts          types shared by main and renderer
  main/
    index.ts               window + IPC
    library.ts             runs providers, merges favorites / hidden / custom games
    actions.ts             opens launcher links or starts programs (allow-listed schemes only)
    store.ts               JSON settings file in the user data folder
    platform/              VDF parser, registry reader, fs helpers
    providers/             one file per launcher: detect() + scan()
  preload/index.ts         the `window.asuna` API
  renderer/                React UI
test/                      vitest suites with on-disk fixtures and a fake registry
```

### Adding a launcher

Create `src/main/providers/<name>.ts` that exports a `Provider` (`detect`, `scan`, `open`), add it to
`providers/index.ts`, add its colors to `renderer/src/launchers.ts`, and add a fixture test.

## Security notes

The renderer never sends commands or paths to run. It sends a game key and an action name, and the main
process looks up the action the provider built. Links are only opened for known launcher schemes.
