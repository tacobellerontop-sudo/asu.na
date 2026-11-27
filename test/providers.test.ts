import { join } from 'node:path'
import { describe, expect, it } from 'vitest'
import { steam } from '../src/main/providers/steam'
import { epic } from '../src/main/providers/epic'
import { gog } from '../src/main/providers/gog'
import { ubisoft } from '../src/main/providers/ubisoft'
import { battlenet } from '../src/main/providers/battlenet'
import { ea } from '../src/main/providers/ea'
import { xbox } from '../src/main/providers/xbox'
import { fakeRegistry, makeCtx, tempTree } from './helpers'

const U = 'HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall'

describe('steam provider', () => {
  const root = tempTree({
    'steamapps/libraryfolders.vdf': '"libraryfolders" { "0" { "path" "MAIN" } }',
    'steamapps/appmanifest_570.acf': '"AppState" { "appid" "570" "name" "Dota 2" "StateFlags" "4" "installdir" "dota 2 beta" "SizeOnDisk" "42" }',
    'steamapps/appmanifest_228980.acf': '"AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" "StateFlags" "4" }',
    'steamapps/appmanifest_1493710.acf': '"AppState" { "appid" "1493710" "name" "Proton Experimental" "StateFlags" "4" }',
    'userdata/123/config/localconfig.vdf': `"UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
      "570" { "LastPlayed" "1700000000" "Playtime" "600" }
      "620" { "LastPlayed" "1600000000" "Playtime" "95" }
      "999" { "Playtime" "1" }
    } } } } }`
  })
  // Second library on another "drive"
  const lib2 = tempTree({
    'steamapps/appmanifest_620.acf': '"AppState" { "appid" "620" "name" "Portal 2" "StateFlags" "6" "installdir" "Portal 2" }'
  })
  // Rewrite the library list now that we know the temp paths.
  tempTreeWrite(join(root, 'steamapps/libraryfolders.vdf'), `"libraryfolders" { "0" { "path" "${esc(root)}" } "1" { "path" "${esc(lib2)}" } }`)

  it('finds installed games across libraries, skips runtimes, adds owned games', async () => {
    const lookups: string[] = []
    const ctx = makeCtx({
      platform: 'linux',
      pathOverride: root,
      fetchJson: async (url) => {
        const id = /appids=(\d+)/.exec(url)![1]
        lookups.push(id)
        return { [id]: { success: true, data: { name: 'Some Soundtrack', type: 'music' } } }
      }
    })
    const d = await steam.detect(ctx)
    expect(d).toEqual({ detected: true, path: root })
    const games = await steam.scan(ctx, d)
    const byId = Object.fromEntries(games.map((g) => [g.id, g]))
    expect(Object.keys(byId).sort()).toEqual(['570', '620'])
    expect(byId['570']).toMatchObject({ title: 'Dota 2', installed: true, sizeBytes: 42, playtimeMinutes: 600, lastPlayed: 1700000000000 })
    expect(byId['570'].installPath).toBe(join(root, 'steamapps', 'common', 'dota 2 beta'))
    expect(byId['620'].installed).toBe(true)
    expect(byId['570'].actions.launch).toEqual({ kind: 'uri', uri: 'steam://rungameid/570' })
    // 999 was looked up, turned out not to be a game, and is cached
    expect(lookups).toEqual(['999'])
    await steam.scan(ctx, d)
    expect(lookups).toEqual(['999'])
  })

  it('adds owned-but-uninstalled games once their name resolves', async () => {
    const ctx = makeCtx({
      platform: 'linux',
      pathOverride: root,
      fetchJson: async () => ({ '999': { success: true, data: { name: 'Hidden Gem', type: 'game' } } })
    })
    const games = await steam.scan(ctx, await steam.detect(ctx))
    expect(games.find((g) => g.id === '999')).toMatchObject({ title: 'Hidden Gem', installed: false })
  })

  it('reports not detected when there is no Steam folder', async () => {
    const d = await steam.detect(makeCtx({ platform: 'linux' }))
    expect(d.detected).toBe(false)
  })
})

describe('epic provider', () => {
  it('reads manifests and the owned catalog', async () => {
    const catalog = [
      { id: 'cat1', namespace: 'ns1', title: 'Installed Game', categories: [{ path: 'games' }], releaseInfo: [{ appId: 'App1', platform: ['Windows'] }], keyImages: [{ type: 'DieselGameBoxTall', url: 'https://img/tall.jpg' }] },
      { id: 'cat2', namespace: 'ns2', title: 'Owned Game', categories: [{ path: 'games' }], releaseInfo: [{ appId: 'App2', platform: ['Windows'] }] },
      { id: 'cat3', namespace: 'ns2', title: 'Some DLC', categories: [{ path: 'games' }], mainGameItem: { id: 'cat2' }, releaseInfo: [{ appId: 'App3' }] },
      { id: 'cat4', namespace: 'ns4', title: 'Mac only', categories: [{ path: 'games' }], releaseInfo: [{ appId: 'App4', platform: ['Mac'] }] }
    ]
    const pd = tempTree({
      'Epic/EpicGamesLauncher/Data/Manifests/a.item': JSON.stringify({ AppName: 'App1', DisplayName: 'Installed Game', CatalogNamespace: 'ns1', CatalogItemId: 'cat1', InstallLocation: 'C:\\Epic\\Game', InstallSize: 10, AppCategories: ['public', 'games'] }),
      'Epic/EpicGamesLauncher/Data/Manifests/b.item': JSON.stringify({ AppName: 'UE_5', DisplayName: 'Unreal Engine', AppCategories: ['engines'] }),
      'Epic/EpicGamesLauncher/Data/Manifests/c.item': '{broken'
    })
    const la = tempTree({
      'EpicGamesLauncher/Saved/Data/Catalog/catcache.bin': Buffer.from(JSON.stringify(catalog)).toString('base64')
    })
    const ctx = makeCtx({ env: { ProgramData: pd, LOCALAPPDATA: la } })
    const d = await epic.detect(ctx)
    expect(d.detected).toBe(true)
    const games = await epic.scan(ctx, d)
    expect(games.map((g) => [g.id, g.installed])).toEqual([
      ['App1', true],
      ['App2', false]
    ])
    expect(games[0].art?.cover).toBe('https://img/tall.jpg')
    expect(games[0].actions.launch).toEqual({ kind: 'uri', uri: 'com.epicgames.launcher://apps/ns1%3Acat1%3AApp1?action=launch&silent=true' })
    expect(games[1].actions.install).toEqual({ kind: 'uri', uri: 'com.epicgames.launcher://apps/ns2%3Acat2%3AApp2?action=install' })
  })
})

describe('registry based providers', () => {
  it('gog: lists games, skips DLC, launches through Galaxy', async () => {
    const galaxyDir = tempTree({ 'GalaxyClient.exe': '' })
    const gameDir = tempTree({ 'unins000.exe': '' })
    const registry = fakeRegistry([
      { path: 'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\GalaxyClient\\paths', values: { client: galaxyDir } },
      { path: 'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1', values: { gameID: '1', gameName: 'The Witcher', path: gameDir, exe: join(gameDir, 'witcher.exe') } },
      { path: 'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\2', values: { gameID: '2', gameName: 'Witcher DLC', dependsOn: '1' } }
    ])
    const ctx = makeCtx({ registry })
    const d = await gog.detect(ctx)
    expect(d).toEqual({ detected: true, path: join(galaxyDir, 'GalaxyClient.exe') })
    const games = await gog.scan(ctx, d)
    expect(games).toHaveLength(1)
    expect(games[0].actions.launch).toEqual({ kind: 'exec', file: join(galaxyDir, 'GalaxyClient.exe'), args: ['/command=runGame', '/gameId=1', `/path=${gameDir}`] })
    expect(games[0].actions.uninstall).toEqual({ kind: 'exec', file: join(gameDir, 'unins000.exe') })
  })

  it('ubisoft: names games from uninstall entries', async () => {
    const registry = fakeRegistry([
      { path: 'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher\\Installs\\5595', values: { InstallDir: 'C:/Ubisoft/AC Valhalla/' } },
      { path: 'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Ubisoft\\Launcher\\Installs\\46', values: { InstallDir: 'C:/Ubisoft/Far Cry 3/' } },
      { path: `${U.replace('HKLM', 'HKEY_LOCAL_MACHINE')}\\Uplay Install 5595`, values: { DisplayName: "Assassin's Creed Valhalla" } }
    ])
    const games = await ubisoft.scan(makeCtx({ registry }), { detected: true })
    expect(games.map((g) => g.title).sort()).toEqual(["Assassin's Creed Valhalla", 'Far Cry 3'])
    expect(games.find((g) => g.id === '5595')!.actions.launch).toEqual({ kind: 'uri', uri: 'uplay://launch/5595/0' })
  })

  it('battlenet: marks installed products and keeps the rest as catalog', async () => {
    const registry = fakeRegistry([
      { path: `${U}\\Overwatch`, values: { DisplayName: 'Overwatch', Publisher: 'Blizzard Entertainment', InstallLocation: 'C:\\Games\\Overwatch', UninstallString: '"C:\\ProgramData\\Battle.net\\Agent\\Blizzard Uninstaller.exe" --lang=enUS --uid=prometheus' } },
      { path: `${U}\\Diablo IV`, values: { DisplayName: 'Diablo IV', Publisher: 'Someone Else' } }
    ])
    const games = await battlenet.scan(makeCtx({ registry }), { detected: true, path: 'C:\\BNet\\Battle.net.exe' })
    const ow = games.find((g) => g.id === 'Pro')!
    expect(ow).toMatchObject({ installed: true, catalogOnly: false, installPath: 'C:\\Games\\Overwatch' })
    expect(ow.actions.launch).toEqual({ kind: 'exec', file: 'C:\\BNet\\Battle.net.exe', args: ['--exec=launch Pro'] })
    expect(ow.actions.uninstall).toMatchObject({ file: 'C:\\ProgramData\\Battle.net\\Agent\\Blizzard Uninstaller.exe' })
    expect(games.find((g) => g.id === 'Fen')).toMatchObject({ installed: false, catalogOnly: true })
  })

  it('ea: only lists games with installer data', async () => {
    const withData = tempTree({ '__Installer/installerdata.xml': '<contentIDs><contentID>Origin.OFR.50.0002694</contentID></contentIDs>' })
    const registry = fakeRegistry([
      { path: `${U}\\{apex}`, values: { DisplayName: 'Apex Legends', Publisher: 'Electronic Arts', InstallLocation: withData + '\\' } },
      { path: `${U}\\{other}`, values: { DisplayName: 'Not a game', Publisher: 'Electronic Arts', InstallLocation: '/nope' } },
      { path: `${U}\\{eaapp}`, values: { DisplayName: 'EA app', Publisher: 'Electronic Arts', InstallLocation: withData } }
    ])
    const games = await ea.scan(makeCtx({ registry }), { detected: true })
    expect(games).toHaveLength(1)
    expect(games[0]).toMatchObject({ id: 'Origin.OFR.50.0002694', title: 'Apex Legends', installed: true })
    expect(games[0].actions.launch).toEqual({ kind: 'uri', uri: 'origin2://game/launch?offerIds=Origin.OFR.50.0002694&autoDownload=1' })
  })

  it('xbox: keeps GDK games only', async () => {
    const game = tempTree({
      'MicrosoftGame.config': '<Game><ShellVisuals DefaultDisplayName="Forza Horizon 5"/></Game>',
      'AppxManifest.xml': '<Package><Applications><Application Id="ForzaHorizon5" Executable="x.exe"/></Applications></Package>'
    })
    const app = tempTree({ 'AppxManifest.xml': '<Application Id="App"/>' })
    const ps = JSON.stringify([
      { Name: 'Forza', PackageFamilyName: 'Microsoft.Forza_8wekyb3d8bbwe', InstallLocation: game },
      { Name: 'Calc', PackageFamilyName: 'Microsoft.Calc_8wekyb3d8bbwe', InstallLocation: app },
      { Name: 'Broken', PackageFamilyName: 'x', InstallLocation: null }
    ])
    const games = await xbox.scan(makeCtx({ powershell: async () => ps }), { detected: true })
    expect(games).toHaveLength(1)
    expect(games[0].title).toBe('Forza Horizon 5')
    expect(games[0].actions.launch).toEqual({ kind: 'exec', file: 'explorer.exe', args: ['shell:AppsFolder\\Microsoft.Forza_8wekyb3d8bbwe!ForzaHorizon5'] })
  })
})

import { writeFileSync } from 'node:fs'
function tempTreeWrite(p: string, s: string) {
  writeFileSync(p, s)
}
function esc(p: string) {
  return p.replace(/\\/g, '\\\\')
}
