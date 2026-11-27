// Stand-in for the Electron preload API so the UI can run in a plain browser (`npm run web`).
import type { AsunaApi, DetectedGame, Game, LauncherId, LauncherStatus, LibrarySnapshot } from '../../shared/types'

const steamArt = (id: number) => ({
  cover: `https://cdn.cloudflare.steamstatic.com/steam/apps/${id}/library_600x900.jpg`,
  header: `https://cdn.cloudflare.steamstatic.com/steam/apps/${id}/header.jpg`
})
const day = 86_400_000
const now = Date.now()
const noop = { launch: { kind: 'uri' as const, uri: 'steam://' }, install: { kind: 'uri' as const, uri: 'steam://' } }

const detected: DetectedGame[] = [
  { launcher: 'steam', id: '1245620', title: 'Elden Ring', installed: true, sizeBytes: 52e9, lastPlayed: now - day, playtimeMinutes: 7340, art: steamArt(1245620), installPath: 'D:\\SteamLibrary\\steamapps\\common\\ELDEN RING', actions: noop },
  { launcher: 'steam', id: '1086940', title: "Baldur's Gate 3", installed: true, sizeBytes: 128e9, lastPlayed: now - 3 * day, playtimeMinutes: 9100, art: steamArt(1086940), actions: noop },
  { launcher: 'steam', id: '413150', title: 'Stardew Valley', installed: false, playtimeMinutes: 4200, lastPlayed: now - 90 * day, art: steamArt(413150), actions: noop },
  { launcher: 'steam', id: '1145360', title: 'Hades', installed: true, sizeBytes: 15e9, playtimeMinutes: 1900, art: steamArt(1145360), actions: noop },
  { launcher: 'steam', id: '292030', title: 'The Witcher 3: Wild Hunt', installed: false, playtimeMinutes: 6100, art: steamArt(292030), actions: noop },
  { launcher: 'steam', id: '367520', title: 'Hollow Knight', installed: true, sizeBytes: 9e9, art: steamArt(367520), actions: noop },
  { launcher: 'epic', id: 'Fortnite', title: 'Fortnite', installed: true, sizeBytes: 31e9, actions: noop },
  { launcher: 'epic', id: 'Sugar', title: 'Rocket League', installed: false, actions: noop },
  { launcher: 'epic', id: 'Kinglet', title: 'Control', installed: false, actions: noop },
  { launcher: 'gog', id: '1207664663', title: 'Cyberpunk 2077', installed: true, sizeBytes: 70e9, lastPlayed: now - 12 * day, actions: noop },
  { launcher: 'battlenet', id: 'Pro', title: 'Overwatch 2', installed: true, sizeBytes: 50e9, lastPlayed: now - 2 * day, actions: noop },
  { launcher: 'battlenet', id: 'Fen', title: 'Diablo IV', installed: false, catalogOnly: true, actions: noop },
  { launcher: 'battlenet', id: 'WTCG', title: 'Hearthstone', installed: false, catalogOnly: true, actions: noop },
  { launcher: 'ea', id: 'Origin.OFR.50.0002694', title: 'Apex Legends', installed: true, sizeBytes: 75e9, actions: noop },
  { launcher: 'ubisoft', id: '5595', title: "Assassin's Creed Valhalla", installed: true, actions: noop },
  { launcher: 'xbox', id: 'Microsoft.254428597CFE2_8wekyb3d8bbwe', title: 'Forza Horizon 5', installed: true, actions: noop }
]

const launchers: LauncherStatus[] = (
  [
    ['steam', 'Steam', true],
    ['epic', 'Epic Games', true],
    ['gog', 'GOG Galaxy', true],
    ['battlenet', 'Battle.net', true],
    ['ea', 'EA app', true],
    ['ubisoft', 'Ubisoft Connect', true],
    ['xbox', 'Xbox / Microsoft Store', true],
    ['custom', 'Added by you', true]
  ] as [LauncherId, string, boolean][]
).map(([id, name, detectedFlag]) => {
  const mine = detected.filter((g) => g.launcher === id)
  return {
    id,
    name,
    supported: true,
    enabled: true,
    detected: detectedFlag,
    path: id === 'steam' ? 'C:\\Program Files (x86)\\Steam' : undefined,
    installedCount: mine.filter((g) => g.installed).length,
    ownedCount: mine.filter((g) => !g.catalogOnly).length,
    downloadUrl: 'https://example.com'
  }
})

let favorites = new Set(['steam:1245620', 'gog:1207664663'])
let hidden = new Set<string>()

function snap(): LibrarySnapshot {
  const games: Game[] = detected
    .map((g) => {
      const key = `${g.launcher}:${g.id}`
      return { ...g, key, favorite: favorites.has(key), hidden: hidden.has(key) }
    })
    .sort((a, b) => a.title.localeCompare(b.title))
  return { games, launchers, scannedAt: Date.now(), platform: 'win32' }
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms))

export const mockApi: AsunaApi = {
  getLibrary: async () => (await wait(200), snap()),
  refresh: async () => (await wait(600), snap()),
  runAction: async (key, action) => console.info('[mock] action', action, key),
  setFavorite: async (key, on) => {
    favorites = new Set(favorites)
    on ? favorites.add(key) : favorites.delete(key)
  },
  setHidden: async (key, on) => {
    hidden = new Set(hidden)
    on ? hidden.add(key) : hidden.delete(key)
  },
  setLauncherEnabled: async (id, on) => {
    const l = launchers.find((x) => x.id === id)
    if (l) l.enabled = on
    return snap()
  },
  openLauncher: async (id) => console.info('[mock] open launcher', id),
  getLauncher: async (id) => console.info('[mock] get launcher', id),
  searchStore: async (term) => {
    await wait(300)
    return [
      { launcher: 'steam', id: '1145350', title: `Hades II (search: ${term})`, price: '29.99 USD', image: 'https://cdn.cloudflare.steamstatic.com/steam/apps/1145350/header.jpg' },
      { launcher: 'steam', id: '570', title: 'Dota 2', price: 'Free', image: 'https://cdn.cloudflare.steamstatic.com/steam/apps/570/header.jpg' }
    ]
  },
  installFromStore: async (l, id) => console.info('[mock] install', l, id),
  pickExecutable: async () => 'C:\\Games\\MyGame\\game.exe',
  addCustomGame: async (input) => {
    detected.push({ launcher: 'custom', id: String(Date.now()), title: input.title, installed: true, actions: noop })
    return snap()
  },
  removeCustomGame: async (key) => {
    const i = detected.findIndex((g) => `${g.launcher}:${g.id}` === key)
    if (i >= 0) detected.splice(i, 1)
    return snap()
  }
}
