import { execFile } from 'node:child_process'
import { homedir } from 'node:os'
import type { DetectedGame, Game, LauncherStatus, LibrarySnapshot } from '../shared/types'
import { readRegistry } from './platform/registry'
import { PROVIDERS, type ProviderContext } from './providers'
import type { CustomGame, Store } from './store'

function powershell(script: string): Promise<string> {
  return new Promise((resolve) => {
    if (process.platform !== 'win32') return resolve('')
    execFile(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-Command', script],
      { windowsHide: true, maxBuffer: 64 * 1024 * 1024, timeout: 60_000 },
      (err, stdout) => resolve(err ? '' : stdout)
    )
  })
}

async function fetchJson(url: string): Promise<unknown> {
  const res = await fetch(url, { signal: AbortSignal.timeout(10_000) })
  if (!res.ok) throw new Error(`HTTP ${res.status}`)
  return res.json()
}

export function customToDetected(c: CustomGame): DetectedGame {
  const args = c.args?.match(/"[^"]*"|\S+/g)?.map((a) => a.replace(/^"|"$/g, '')) ?? []
  const dir = c.executable.replace(/[\\/][^\\/]*$/, '')
  return {
    launcher: 'custom',
    id: c.id,
    title: c.title,
    installed: true,
    installPath: dir,
    actions: { launch: { kind: 'exec', file: c.executable, args, cwd: dir } }
  }
}

export class Library {
  private snapshot: LibrarySnapshot | null = null
  private scanning: Promise<LibrarySnapshot> | null = null

  constructor(private store: Store) {}

  get current(): LibrarySnapshot | null {
    return this.snapshot
  }

  find(key: string): Game | undefined {
    return this.snapshot?.games.find((g) => g.key === key)
  }

  /** Re-applies favorites / hidden / custom games without rescanning the disk. */
  remerge(): LibrarySnapshot {
    if (!this.snapshot) throw new Error('library not scanned yet')
    const detected = this.snapshot.games.filter((g) => g.launcher !== 'custom')
    this.snapshot = this.build(detected, this.snapshot.launchers)
    return this.snapshot
  }

  scan(): Promise<LibrarySnapshot> {
    this.scanning ??= this.doScan().finally(() => (this.scanning = null))
    return this.scanning
  }

  private async doScan(): Promise<LibrarySnapshot> {
    const { disabledLaunchers, pathOverrides } = this.store.data
    const results = await Promise.all(
      PROVIDERS.map(async (p) => {
        const supported = p.platforms.includes(process.platform)
        const enabled = !disabledLaunchers.includes(p.id)
        const status: LauncherStatus = {
          id: p.id,
          name: p.name,
          supported,
          enabled,
          detected: false,
          installedCount: 0,
          ownedCount: 0,
          downloadUrl: p.downloadUrl
        }
        if (!supported || !enabled) return { status, games: [] as DetectedGame[] }
        const ctx: ProviderContext = {
          platform: process.platform,
          env: process.env,
          home: homedir(),
          registry: readRegistry,
          powershell,
          fetchJson,
          cache: this.store,
          pathOverride: pathOverrides[p.id]
        }
        try {
          const detection = await p.detect(ctx)
          status.detected = detection.detected
          status.path = detection.path
          if (!detection.detected) return { status, games: [] }
          const games = await p.scan(ctx, detection)
          status.installedCount = games.filter((g) => g.installed).length
          status.ownedCount = games.filter((g) => !g.catalogOnly).length
          return { status, games }
        } catch (err) {
          status.error = err instanceof Error ? err.message : String(err)
          return { status, games: [] }
        }
      })
    )
    const custom: LauncherStatus = {
      id: 'custom',
      name: 'Added by you',
      supported: true,
      enabled: true,
      detected: true,
      installedCount: this.store.data.customGames.length,
      ownedCount: this.store.data.customGames.length
    }
    this.snapshot = this.build(
      results.flatMap((r) => r.games),
      [...results.map((r) => r.status), custom]
    )
    return this.snapshot
  }

  private build(detected: DetectedGame[], launchers: LauncherStatus[]): LibrarySnapshot {
    const { favorites, hidden, lastPlayed, customGames } = this.store.data
    const fav = new Set(favorites)
    const hid = new Set(hidden)
    const all = [...detected, ...customGames.map(customToDetected)]
    const games: Game[] = all.map((g) => {
      const key = `${g.launcher}:${g.id}`
      const ours = lastPlayed[key]
      return {
        ...g,
        key,
        favorite: fav.has(key),
        hidden: hid.has(key),
        lastPlayed: Math.max(g.lastPlayed ?? 0, ours ?? 0) || undefined
      }
    })
    games.sort((a, b) => a.title.localeCompare(b.title, undefined, { sensitivity: 'base' }))
    const customStatus = launchers.find((l) => l.id === 'custom')
    if (customStatus) customStatus.installedCount = customStatus.ownedCount = customGames.length
    return { games, launchers, scannedAt: Date.now(), platform: process.platform }
  }
}
