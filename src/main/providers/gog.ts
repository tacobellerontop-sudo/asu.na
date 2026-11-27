import { join } from 'node:path'
import type { DetectedGame, LaunchAction } from '../../shared/types'
import { childKeys, regValue } from '../platform/registry'
import { exists, firstExisting } from '../platform/fsx'
import type { Provider } from './types'

const GAMES_KEY = 'HKLM\\SOFTWARE\\WOW6432Node\\GOG.com\\Games'
const CLIENT_KEY = 'HKLM\\SOFTWARE\\WOW6432Node\\GOG.com\\GalaxyClient\\paths'

export const gog: Provider = {
  id: 'gog',
  name: 'GOG Galaxy',
  platforms: ['win32'],
  downloadUrl: 'https://www.gog.com/galaxy',

  async detect(ctx) {
    const [paths] = await ctx.registry(CLIENT_KEY)
    const dir = await firstExisting([
      ctx.pathOverride,
      regValue(paths, 'client'),
      join(ctx.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)', 'GOG Galaxy')
    ])
    const exe = dir ? join(dir, 'GalaxyClient.exe') : undefined
    if (exe && (await exists(exe))) return { detected: true, path: exe }
    // Games installed from offline installers still register under GOG.com\Games.
    const games = await ctx.registry(GAMES_KEY)
    return { detected: games.length > 0 }
  },

  open: (d) => (d.path ? { kind: 'exec', file: d.path } : { kind: 'uri', uri: 'goggalaxy://openStoreUrl/www.gog.com' }),

  async scan(ctx, detection) {
    const all = await ctx.registry(GAMES_KEY)
    const galaxy = detection.path
    const games: DetectedGame[] = []
    for (const key of childKeys(all, GAMES_KEY)) {
      const id = regValue(key, 'gameID') ?? key.name
      const title = regValue(key, 'gameName')
      const path = regValue(key, 'path')
      const exe = regValue(key, 'exe')
      // DLCs point at their base game through dependsOn.
      if (!title || regValue(key, 'dependsOn')) continue

      let launch: LaunchAction | undefined
      if (galaxy && path) {
        launch = { kind: 'exec', file: galaxy, args: ['/command=runGame', `/gameId=${id}`, `/path=${path}`] }
      } else if (exe) {
        launch = { kind: 'exec', file: exe, cwd: regValue(key, 'workingDir') || path }
      }
      const uninstaller = path ? join(path, 'unins000.exe') : undefined
      games.push({
        launcher: 'gog',
        id,
        title,
        installed: true,
        installPath: path,
        actions: {
          launch,
          install: { kind: 'uri', uri: `goggalaxy://openGameView/${id}` },
          uninstall: uninstaller && (await exists(uninstaller)) ? { kind: 'exec', file: uninstaller } : undefined,
          store: { kind: 'uri', uri: `goggalaxy://openGameView/${id}` }
        }
      })
    }
    return games
  }
}
