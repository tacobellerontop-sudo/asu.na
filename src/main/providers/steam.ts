import { join } from 'node:path'
import type { DetectedGame } from '../../shared/types'
import { asObject, asString, getCI, parseVdf, type VdfObject } from '../platform/vdf'
import { regValue } from '../platform/registry'
import { exists, firstExisting, listDir, mapLimit, readText } from '../platform/fsx'
import type { Provider, ProviderContext } from './types'

const CDN = 'https://cdn.cloudflare.steamstatic.com/steam/apps'

/** Runtimes and redistributables that show up as apps but are not games. */
const NON_GAME_IDS = new Set(['228980', '1070560', '1391110', '1628350', '1493710', '2180100', '1826330'])
const NON_GAME_NAME = /^(proton|steam linux runtime|steamworks|steamvr)\b/i

/** Lookups per scan for names of games that are owned but not installed. Cached forever after. */
const MAX_NAME_LOOKUPS = 80

interface AppName {
  name: string
  isGame: boolean
}

export function steamArt(appid: string) {
  return { cover: `${CDN}/${appid}/library_600x900.jpg`, header: `${CDN}/${appid}/header.jpg` }
}

export function steamActions(appid: string) {
  return {
    launch: { kind: 'uri' as const, uri: `steam://rungameid/${appid}` },
    install: { kind: 'uri' as const, uri: `steam://install/${appid}` },
    uninstall: { kind: 'uri' as const, uri: `steam://uninstall/${appid}` },
    store: { kind: 'uri' as const, uri: `steam://store/${appid}` }
  }
}

/** Every library folder (including the main install) listed in libraryfolders.vdf. */
export function parseLibraryFolders(text: string, steamRoot: string): string[] {
  const root = asObject(getCI(parseVdf(text), 'libraryfolders')) ?? {}
  const folders = new Set<string>([steamRoot])
  for (const entry of Object.values(root)) {
    // New format: "0" { "path" "D:\\SteamLibrary" ... }   Old format: "1" "D:\\SteamLibrary"
    const p = typeof entry === 'string' ? entry : asString(getCI(entry, 'path'))
    if (p && /[\\/]/.test(p)) folders.add(p)
  }
  return [...folders]
}

export interface AppManifest {
  appid: string
  name: string
  installdir?: string
  sizeOnDisk?: number
  lastPlayed?: number
  fullyInstalled: boolean
}

export function parseAppManifest(text: string): AppManifest | null {
  const state = asObject(getCI(parseVdf(text), 'AppState'))
  if (!state) return null
  const appid = asString(getCI(state, 'appid'))
  const name = asString(getCI(state, 'name'))
  if (!appid || !name) return null
  const flags = Number(asString(getCI(state, 'StateFlags')) ?? '4')
  const size = Number(asString(getCI(state, 'SizeOnDisk')))
  const lastPlayed = Number(asString(getCI(state, 'LastPlayed')))
  return {
    appid,
    name,
    installdir: asString(getCI(state, 'installdir')),
    sizeOnDisk: Number.isFinite(size) && size > 0 ? size : undefined,
    lastPlayed: Number.isFinite(lastPlayed) && lastPlayed > 0 ? lastPlayed * 1000 : undefined,
    // StateFlags bit 4 = fully installed. Anything else is updating / partially downloaded.
    fullyInstalled: (flags & 4) === 4
  }
}

/** Per-app play stats from userdata/<id>/config/localconfig.vdf. Covers games that are not installed too. */
export function parseLocalConfigApps(text: string): Map<string, { lastPlayed?: number; playtime?: number }> {
  const apps = asObject(getCI(parseVdf(text), 'UserLocalConfigStore', 'Software', 'Valve', 'Steam', 'apps')) ?? {}
  const out = new Map<string, { lastPlayed?: number; playtime?: number }>()
  for (const [appid, v] of Object.entries(apps)) {
    if (!/^\d+$/.test(appid) || typeof v !== 'object') continue
    const lp = Number(asString(getCI(v, 'LastPlayed')))
    const pt = Number(asString(getCI(v, 'Playtime')))
    out.set(appid, {
      lastPlayed: lp > 0 ? lp * 1000 : undefined,
      playtime: pt > 0 ? pt : undefined
    })
  }
  return out
}

async function findSteamRoot(ctx: ProviderContext): Promise<string | undefined> {
  if (ctx.pathOverride) return ctx.pathOverride
  const h = ctx.home
  if (ctx.platform === 'win32') {
    const [key] = await ctx.registry('HKCU\\Software\\Valve\\Steam')
    const fromReg = regValue(key, 'SteamPath')?.replace(/\//g, '\\')
    return firstExisting([
      fromReg,
      join(ctx.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)', 'Steam'),
      join(ctx.env.ProgramFiles ?? 'C:\\Program Files', 'Steam')
    ])
  }
  if (ctx.platform === 'darwin') return firstExisting([join(h, 'Library/Application Support/Steam')])
  return firstExisting([
    join(h, '.steam/steam'),
    join(h, '.local/share/Steam'),
    join(h, '.var/app/com.valvesoftware.Steam/.local/share/Steam')
  ])
}

async function resolveName(ctx: ProviderContext, appid: string): Promise<AppName | null> {
  const cacheKey = `steam.appname.${appid}`
  const cached = ctx.cache.get<AppName>(cacheKey)
  if (cached) return cached
  try {
    const data = (await ctx.fetchJson(
      `https://store.steampowered.com/api/appdetails?appids=${appid}&filters=basic`
    )) as Record<string, { success: boolean; data?: { name: string; type: string } }>
    const entry = data[appid]
    const result: AppName = entry?.success && entry.data
      ? { name: entry.data.name, isGame: entry.data.type === 'game' }
      : { name: '', isGame: false } // delisted or region locked: remember so we stop asking
    ctx.cache.set(cacheKey, result)
    return result
  } catch {
    return null // offline: try again next scan
  }
}

export const steam: Provider = {
  id: 'steam',
  name: 'Steam',
  platforms: ['win32', 'darwin', 'linux'],
  downloadUrl: 'https://store.steampowered.com/about/',

  async detect(ctx) {
    const path = await findSteamRoot(ctx)
    return { detected: !!path && (await exists(join(path, 'steamapps'))), path }
  },

  open: () => ({ kind: 'uri', uri: 'steam://open/games' }),

  async scan(ctx, detection) {
    const root = detection.path
    if (!root) return []
    const vdf = await readText(join(root, 'steamapps', 'libraryfolders.vdf'))
    const libraries = vdf ? parseLibraryFolders(vdf, root) : [root]

    // Play stats for every Steam user that has signed in on this machine.
    const stats = new Map<string, { lastPlayed?: number; playtime?: number }>()
    for (const user of await listDir(join(root, 'userdata'))) {
      const text = await readText(join(root, 'userdata', user, 'config', 'localconfig.vdf'))
      if (!text) continue
      for (const [id, s] of parseLocalConfigApps(text)) {
        const prev = stats.get(id)
        stats.set(id, {
          lastPlayed: Math.max(prev?.lastPlayed ?? 0, s.lastPlayed ?? 0) || undefined,
          playtime: Math.max(prev?.playtime ?? 0, s.playtime ?? 0) || undefined
        })
      }
    }

    const games = new Map<string, DetectedGame>()
    for (const lib of libraries) {
      const appsDir = join(lib.replace(/\\\\/g, '\\'), 'steamapps')
      for (const file of await listDir(appsDir)) {
        if (!/^appmanifest_\d+\.acf$/i.test(file)) continue
        const text = await readText(join(appsDir, file))
        const m = text ? parseAppManifest(text) : null
        if (!m || NON_GAME_IDS.has(m.appid) || NON_GAME_NAME.test(m.name)) continue
        const s = stats.get(m.appid)
        games.set(m.appid, {
          launcher: 'steam',
          id: m.appid,
          title: m.name,
          installed: m.fullyInstalled,
          installPath: m.installdir ? join(appsDir, 'common', m.installdir) : undefined,
          sizeBytes: m.sizeOnDisk,
          lastPlayed: s?.lastPlayed ?? m.lastPlayed,
          playtimeMinutes: s?.playtime,
          art: steamArt(m.appid),
          actions: steamActions(m.appid)
        })
      }
    }

    // Games this account has played before but which are not installed here.
    const missing = [...stats.keys()].filter((id) => !games.has(id) && !NON_GAME_IDS.has(id))
    let lookups = 0
    const names = await mapLimit(missing, 4, async (id) => {
      const cached = ctx.cache.get<AppName>(`steam.appname.${id}`)
      if (cached) return cached
      if (lookups++ >= MAX_NAME_LOOKUPS) return null
      return resolveName(ctx, id)
    })
    missing.forEach((id, i) => {
      const n = names[i]
      if (!n?.isGame || !n.name) return
      const s = stats.get(id)
      games.set(id, {
        launcher: 'steam',
        id,
        title: n.name,
        installed: false,
        lastPlayed: s?.lastPlayed,
        playtimeMinutes: s?.playtime,
        art: steamArt(id),
        actions: steamActions(id)
      })
    })

    return [...games.values()]
  }
}
