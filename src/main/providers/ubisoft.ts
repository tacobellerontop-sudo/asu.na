import { join, win32 } from 'node:path'
import type { DetectedGame } from '../../shared/types'
import { UNINSTALL_KEYS, childKeys, regValue } from '../platform/registry'
import { firstExisting } from '../platform/fsx'
import type { Provider } from './types'

const LAUNCHER_KEY = 'HKLM\\SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher'
const INSTALLS_KEY = `${LAUNCHER_KEY}\\Installs`

export function ubisoftActions(id: string) {
  return {
    launch: { kind: 'uri' as const, uri: `uplay://launch/${id}/0` },
    install: { kind: 'uri' as const, uri: `uplay://install/${id}` },
    uninstall: { kind: 'uri' as const, uri: `uplay://uninstall/${id}` },
    store: { kind: 'uri' as const, uri: `uplay://open/game/${id}` }
  }
}

export const ubisoft: Provider = {
  id: 'ubisoft',
  name: 'Ubisoft Connect',
  platforms: ['win32'],
  downloadUrl: 'https://ubisoftconnect.com/',

  async detect(ctx) {
    const all = await ctx.registry(LAUNCHER_KEY)
    const root = all.find((k) => k.name.toLowerCase() === 'launcher')
    const dir = await firstExisting([
      ctx.pathOverride,
      regValue(root, 'InstallDir'),
      join(ctx.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)', 'Ubisoft', 'Ubisoft Game Launcher')
    ])
    return { detected: !!dir, path: dir }
  },

  open: () => ({ kind: 'uri', uri: 'uplay://' }),

  async scan(ctx) {
    const installs = childKeys(await ctx.registry(INSTALLS_KEY), INSTALLS_KEY)
    if (!installs.length) return []
    // Friendly names live in the uninstall entries: "Uplay Install <id>".
    const names = new Map<string, string>()
    for (const root of UNINSTALL_KEYS) {
      for (const k of childKeys(await ctx.registry(root), root)) {
        const m = /^Uplay Install (\d+)$/i.exec(k.name)
        const dn = regValue(k, 'DisplayName')
        if (m && dn) names.set(m[1], dn)
      }
    }
    return installs.map((k): DetectedGame => {
      const dir = regValue(k, 'InstallDir')?.replace(/\//g, '\\').replace(/\\$/, '')
      return {
        launcher: 'ubisoft',
        id: k.name,
        title: names.get(k.name) ?? (dir ? win32.basename(dir) : `Ubisoft game ${k.name}`),
        installed: true,
        installPath: dir,
        actions: ubisoftActions(k.name)
      }
    })
  }
}
