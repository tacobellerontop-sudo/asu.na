import { join } from 'node:path'
import type { DetectedGame, LaunchAction } from '../../shared/types'
import { UNINSTALL_KEYS, childKeys, regValue, splitCommandLine } from '../platform/registry'
import { exists, firstExisting } from '../platform/fsx'
import type { Provider } from './types'

/**
 * Battle.net has no local "owned games" list we can read without the account,
 * so the catalog doubles as the install list. `match` is tested against the
 * DisplayName of the Windows uninstall entry.
 */
export const BATTLENET_CATALOG: { code: string; title: string; match: RegExp }[] = [
  { code: 'WoW', title: 'World of Warcraft', match: /^world of warcraft$/i },
  { code: 'WoWC', title: 'World of Warcraft Classic', match: /^world of warcraft classic/i },
  { code: 'Pro', title: 'Overwatch 2', match: /^overwatch/i },
  { code: 'WTCG', title: 'Hearthstone', match: /^hearthstone/i },
  { code: 'Fen', title: 'Diablo IV', match: /^diablo iv/i },
  { code: 'OSI', title: 'Diablo II: Resurrected', match: /^diablo ii/i },
  { code: 'D3', title: 'Diablo III', match: /^diablo iii/i },
  { code: 'ANBS', title: 'Diablo Immortal', match: /^diablo immortal/i },
  { code: 'Hero', title: 'Heroes of the Storm', match: /^heroes of the storm/i },
  { code: 'S2', title: 'StarCraft II', match: /^starcraft ii/i },
  { code: 'S1', title: 'StarCraft: Remastered', match: /^starcraft( remastered)?$/i },
  { code: 'W3', title: 'Warcraft III: Reforged', match: /^warcraft iii/i },
  { code: 'GRY', title: 'Warcraft Rumble', match: /^warcraft rumble/i },
  { code: 'RTRO', title: 'Blizzard Arcade Collection', match: /^blizzard arcade/i },
  { code: 'Auks', title: 'Call of Duty', match: /^call of duty$/i }
]

export const battlenet: Provider = {
  id: 'battlenet',
  name: 'Battle.net',
  platforms: ['win32'],
  downloadUrl: 'https://download.battle.net/',

  async detect(ctx) {
    const dir = await firstExisting([
      ctx.pathOverride,
      join(ctx.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)', 'Battle.net')
    ])
    const exe = dir ? join(dir, 'Battle.net.exe') : undefined
    return { detected: !!exe && (await exists(exe)), path: exe }
  },

  open: (d) => (d.path ? { kind: 'exec', file: d.path } : { kind: 'uri', uri: 'battlenet://' }),

  async scan(ctx, detection) {
    const installed = new Map<string, { dir?: string; uninstall?: LaunchAction }>()
    for (const root of UNINSTALL_KEYS) {
      for (const k of childKeys(await ctx.registry(root), root)) {
        if (!/blizzard/i.test(regValue(k, 'Publisher') ?? '')) continue
        const name = regValue(k, 'DisplayName') ?? ''
        const entry = BATTLENET_CATALOG.find((c) => c.match.test(name.trim()))
        if (!entry) continue
        const unins = splitCommandLine(regValue(k, 'UninstallString') ?? '')
        installed.set(entry.code, {
          dir: regValue(k, 'InstallLocation'),
          uninstall: unins ? { kind: 'exec', file: unins.file, args: unins.args } : undefined
        })
      }
    }

    const client = detection.path
    return BATTLENET_CATALOG.map((c): DetectedGame => {
      const inst = installed.get(c.code)
      const page: LaunchAction = { kind: 'uri', uri: `battlenet://${c.code}` }
      return {
        launcher: 'battlenet',
        id: c.code,
        title: c.title,
        installed: !!inst,
        catalogOnly: !inst,
        installPath: inst?.dir,
        actions: {
          launch: client ? { kind: 'exec', file: client, args: [`--exec=launch ${c.code}`] } : page,
          install: page,
          uninstall: inst?.uninstall,
          store: page
        }
      }
    })
  }
}
