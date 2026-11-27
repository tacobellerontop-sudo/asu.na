import { join } from 'node:path'
import type { DetectedGame, LaunchAction } from '../../shared/types'
import { UNINSTALL_KEYS, childKeys, regValue, splitCommandLine } from '../platform/registry'
import { exists, firstExisting, readText } from '../platform/fsx'
import type { Provider } from './types'

/** Content ids from <game dir>\__Installer\installerdata.xml, used by EA's launch links. */
export function parseContentIds(xml: string): string[] {
  return [...xml.matchAll(/<contentID>\s*([^<\s]+)\s*<\/contentID>/gi)].map((m) => m[1])
}

const EA_PUBLISHER = /electronic arts|ea games|ea swiss|respawn|bioware|codemasters/i

export const ea: Provider = {
  id: 'ea',
  name: 'EA app',
  platforms: ['win32'],
  downloadUrl: 'https://www.ea.com/ea-app',

  async detect(ctx) {
    const [key] = await ctx.registry('HKLM\\SOFTWARE\\Electronic Arts\\EA Desktop')
    const exe = await firstExisting([
      ctx.pathOverride,
      regValue(key, 'ClientPath'),
      join(ctx.env.ProgramFiles ?? 'C:\\Program Files', 'Electronic Arts', 'EA Desktop', 'EA Desktop', 'EADesktop.exe')
    ])
    return { detected: !!exe, path: exe }
  },

  open: (d) => (d.path ? { kind: 'exec', file: d.path } : { kind: 'uri', uri: 'origin2://library/open' }),

  async scan(ctx) {
    const games: DetectedGame[] = []
    const seen = new Set<string>()
    for (const root of UNINSTALL_KEYS) {
      for (const k of childKeys(await ctx.registry(root), root)) {
        const publisher = regValue(k, 'Publisher') ?? ''
        const title = regValue(k, 'DisplayName')
        const dir = regValue(k, 'InstallLocation')?.replace(/\\$/, '')
        if (!title || !dir || !EA_PUBLISHER.test(publisher)) continue
        if (/^(ea app|ea desktop|origin)\b/i.test(title)) continue
        const xml = await readText(join(dir, '__Installer', 'installerdata.xml'))
        if (!xml) continue // not a game managed by the EA app
        const ids = parseContentIds(xml)
        if (!ids.length || seen.has(ids[0])) continue
        seen.add(ids[0])

        const offer = ids.join(',')
        const unins = splitCommandLine(regValue(k, 'UninstallString') ?? '')
        const uninstall: LaunchAction | undefined =
          unins && (await exists(unins.file)) ? { kind: 'exec', file: unins.file, args: unins.args } : undefined
        games.push({
          launcher: 'ea',
          id: ids[0],
          title,
          installed: true,
          installPath: dir,
          sizeBytes: Number(regValue(k, 'EstimatedSize') ?? 0) * 1024 || undefined,
          actions: {
            launch: { kind: 'uri', uri: `origin2://game/launch?offerIds=${encodeURIComponent(offer)}&autoDownload=1` },
            install: { kind: 'uri', uri: `origin2://game/download?offerId=${encodeURIComponent(ids[0])}` },
            uninstall,
            store: { kind: 'uri', uri: 'origin2://store/open' }
          }
        })
      }
    }
    return games
  }
}
