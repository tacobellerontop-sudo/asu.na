import { join } from 'node:path'
import type { DetectedGame } from '../../shared/types'
import { exists, readText } from '../platform/fsx'
import type { Provider } from './types'

interface AppxPackage {
  Name: string
  PackageFamilyName: string
  InstallLocation: string | null
  IsFramework?: boolean
}

const LIST_PACKAGES =
  'Get-AppxPackage | Where-Object { -not $_.IsFramework -and $_.SignatureKind -eq "Store" } | ' +
  'Select-Object Name, PackageFamilyName, InstallLocation | ConvertTo-Json -Compress'

/** Pulls the display name and executable id out of MicrosoftGame.config. */
export function parseMicrosoftGameConfig(xml: string): { title?: string; storeId?: string } {
  const title = /<ShellVisuals[^>]*DefaultDisplayName="([^"]+)"/i.exec(xml)?.[1]
  const storeId = /<StoreId>\s*([^<\s]+)\s*<\/StoreId>/i.exec(xml)?.[1]
  return { title: title && !title.startsWith('ms-resource:') ? title : undefined, storeId }
}

/** First <Application Id="..."> in AppxManifest.xml, needed for shell:AppsFolder links. */
export function parseAppId(manifest: string): string | undefined {
  return /<Application\s[^>]*Id="([^"]+)"/i.exec(manifest)?.[1]
}

export const xbox: Provider = {
  id: 'xbox',
  name: 'Xbox / Microsoft Store',
  platforms: ['win32'],
  downloadUrl: 'https://www.xbox.com/apps/xbox-app-for-pc',

  async detect(ctx) {
    return { detected: ctx.platform === 'win32' }
  },

  open: () => ({ kind: 'uri', uri: 'xbox:' }),

  async scan(ctx) {
    const raw = await ctx.powershell(LIST_PACKAGES)
    if (!raw.trim()) return []
    let pkgs: AppxPackage[]
    try {
      const parsed = JSON.parse(raw)
      pkgs = Array.isArray(parsed) ? parsed : [parsed]
    } catch {
      return []
    }
    const games: DetectedGame[] = []
    for (const p of pkgs) {
      if (!p.InstallLocation) continue
      // MicrosoftGame.config only ships with games built on the Microsoft GDK (Game Pass / Xbox PC).
      const config = await readText(join(p.InstallLocation, 'MicrosoftGame.config'))
      if (!config) continue
      const { title, storeId } = parseMicrosoftGameConfig(config)
      const manifest = await readText(join(p.InstallLocation, 'AppxManifest.xml'))
      const appId = manifest ? parseAppId(manifest) : undefined
      if (!appId) continue
      const store = storeId
        ? `ms-windows-store://pdp/?ProductId=${storeId}`
        : `ms-windows-store://pdp/?PFN=${p.PackageFamilyName}`
      games.push({
        launcher: 'xbox',
        id: p.PackageFamilyName,
        title: title ?? p.Name,
        installed: true,
        installPath: (await exists(p.InstallLocation)) ? p.InstallLocation : undefined,
        actions: {
          launch: { kind: 'exec', file: 'explorer.exe', args: [`shell:AppsFolder\\${p.PackageFamilyName}!${appId}`] },
          install: { kind: 'uri', uri: store },
          uninstall: { kind: 'uri', uri: 'ms-settings:appsfeatures' },
          store: { kind: 'uri', uri: store }
        }
      })
    }
    return games
  }
}
