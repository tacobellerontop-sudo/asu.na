import { join } from 'node:path'
import type { DetectedGame, GameArt } from '../../shared/types'
import { firstExisting, listDir, readText } from '../platform/fsx'
import type { Provider, ProviderContext } from './types'

const URI = 'com.epicgames.launcher://apps'

export interface EpicManifest {
  AppName: string
  DisplayName: string
  CatalogNamespace?: string
  CatalogItemId?: string
  InstallLocation?: string
  InstallSize?: number
  MainGameAppName?: string
  AppCategories?: string[]
  bIsIncompleteInstall?: boolean
}

interface CatalogItem {
  id: string
  namespace: string
  title: string
  categories?: { path: string }[]
  mainGameItem?: unknown
  releaseInfo?: { appId: string; platform?: string[] }[]
  keyImages?: { type: string; url: string }[]
}

/** Epic deep links use `namespace:catalogItemId:appName`, URL-encoded. */
export function epicAppRef(ns: string | undefined, catalogId: string | undefined, appName: string): string {
  return ns && catalogId ? encodeURIComponent(`${ns}:${catalogId}:${appName}`) : encodeURIComponent(appName)
}

export function epicActions(ref: string) {
  return {
    launch: { kind: 'uri' as const, uri: `${URI}/${ref}?action=launch&silent=true` },
    install: { kind: 'uri' as const, uri: `${URI}/${ref}?action=install` },
    // Epic has no uninstall deep link; send the user to their library where the menu lives.
    uninstall: { kind: 'uri' as const, uri: 'com.epicgames.launcher://library' },
    store: { kind: 'uri' as const, uri: 'com.epicgames.launcher://store' }
  }
}

/** True for base games, false for DLC, add-ons and non-game apps such as Unreal Engine. */
export function isBaseGame(m: EpicManifest): boolean {
  if (m.MainGameAppName && m.MainGameAppName !== m.AppName) return false
  if (m.AppCategories && m.AppCategories.length && !m.AppCategories.includes('games')) return false
  return true
}

function artFrom(images: CatalogItem['keyImages']): GameArt | undefined {
  if (!images?.length) return undefined
  const pick = (...types: string[]) => types.map((t) => images.find((i) => i.type === t)?.url).find(Boolean)
  return {
    cover: pick('DieselGameBoxTall', 'OfferImageTall', 'Thumbnail'),
    header: pick('DieselGameBox', 'OfferImageWide', 'DieselStoreFrontWide')
  }
}

/**
 * The launcher caches the owned catalog as base64-encoded JSON. The format is undocumented,
 * so any failure just means "no extra owned games / art".
 */
export function parseCatalogCache(raw: string): CatalogItem[] {
  try {
    const json = JSON.parse(Buffer.from(raw.trim(), 'base64').toString('utf8'))
    return Array.isArray(json) ? json : []
  } catch {
    return []
  }
}

function dataDirs(ctx: ProviderContext) {
  if (ctx.platform === 'win32') {
    return {
      manifests: join(ctx.env.ProgramData ?? 'C:\\ProgramData', 'Epic', 'EpicGamesLauncher', 'Data', 'Manifests'),
      catcache: join(ctx.env.LOCALAPPDATA ?? join(ctx.home, 'AppData', 'Local'), 'EpicGamesLauncher', 'Saved', 'Data', 'Catalog', 'catcache.bin')
    }
  }
  const base = join(ctx.home, 'Library', 'Application Support', 'Epic', 'EpicGamesLauncher')
  return {
    manifests: join(base, 'Data', 'Manifests'),
    catcache: join(base, 'Saved', 'Data', 'Catalog', 'catcache.bin')
  }
}

export const epic: Provider = {
  id: 'epic',
  name: 'Epic Games',
  platforms: ['win32', 'darwin'],
  downloadUrl: 'https://store.epicgames.com/download',

  async detect(ctx) {
    const path = await firstExisting([ctx.pathOverride, dataDirs(ctx).manifests])
    return { detected: !!path, path }
  },

  open: () => ({ kind: 'uri', uri: 'com.epicgames.launcher://library' }),

  async scan(ctx) {
    const dirs = dataDirs(ctx)
    const manifestDir = ctx.pathOverride ?? dirs.manifests
    const catalogRaw = await readText(dirs.catcache)
    const catalog = catalogRaw ? parseCatalogCache(catalogRaw) : []
    const byId = new Map(catalog.map((c) => [c.id, c]))

    const games = new Map<string, DetectedGame>()
    for (const file of await listDir(manifestDir)) {
      if (!file.endsWith('.item')) continue
      let m: EpicManifest
      try {
        m = JSON.parse((await readText(join(manifestDir, file))) ?? '')
      } catch {
        continue
      }
      if (!m.AppName || !m.DisplayName || !isBaseGame(m)) continue
      games.set(m.AppName, {
        launcher: 'epic',
        id: m.AppName,
        title: m.DisplayName,
        installed: !m.bIsIncompleteInstall,
        installPath: m.InstallLocation,
        sizeBytes: m.InstallSize || undefined,
        art: artFrom(m.CatalogItemId ? byId.get(m.CatalogItemId)?.keyImages : undefined),
        actions: epicActions(epicAppRef(m.CatalogNamespace, m.CatalogItemId, m.AppName))
      })
    }

    // Owned but not installed, from the catalog cache.
    for (const item of catalog) {
      if (item.mainGameItem) continue
      if (!item.categories?.some((c) => c.path === 'games')) continue
      const release = item.releaseInfo?.find((r) => !r.platform || r.platform.includes(ctx.platform === 'darwin' ? 'Mac' : 'Windows'))
      if (!release?.appId || games.has(release.appId)) continue
      games.set(release.appId, {
        launcher: 'epic',
        id: release.appId,
        title: item.title,
        installed: false,
        art: artFrom(item.keyImages),
        actions: epicActions(epicAppRef(item.namespace, item.id, release.appId))
      })
    }
    return [...games.values()]
  }
}
