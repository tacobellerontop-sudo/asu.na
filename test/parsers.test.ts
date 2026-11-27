import { describe, expect, it } from 'vitest'
import { getCI, parseVdf } from '../src/main/platform/vdf'
import { childKeys, parseRegQuery, splitCommandLine } from '../src/main/platform/registry'
import { parseAppManifest, parseLibraryFolders, parseLocalConfigApps } from '../src/main/providers/steam'
import { epicAppRef, isBaseGame, parseCatalogCache } from '../src/main/providers/epic'
import { parseContentIds } from '../src/main/providers/ea'
import { parseAppId, parseMicrosoftGameConfig } from '../src/main/providers/xbox'
import { isAllowedUri } from '../src/main/actions'

describe('vdf', () => {
  it('parses nested objects, escapes and comments', () => {
    const v = parseVdf(`// comment
"Root"
{
  "path"   "C:\\\\Games\\\\Steam"
  "Nested" { "a" "1" "empty" "" }
  bare value
}`)
    expect(getCI(v, 'root', 'PATH')).toBe('C:\\Games\\Steam')
    expect(getCI(v, 'Root', 'nested', 'A')).toBe('1')
    expect(getCI(v, 'Root', 'nested', 'empty')).toBe('')
    expect(getCI(v, 'Root', 'bare')).toBe('value')
  })
})

describe('steam parsers', () => {
  it('reads new and old libraryfolders formats', () => {
    const modern = `"libraryfolders" { "0" { "path" "C:\\\\Steam" "apps" { "10" "1" } } "1" { "path" "D:\\\\SteamLibrary" } }`
    expect(parseLibraryFolders(modern, 'C:\\Steam')).toEqual(['C:\\Steam', 'D:\\SteamLibrary'])
    const legacy = `"LibraryFolders" { "TimeNextStatsReport" "123" "ContentStatsID" "-1" "1" "E:\\\\Games" }`
    expect(parseLibraryFolders(legacy, '/root')).toEqual(['/root', 'E:\\Games'])
  })

  it('reads app manifests and install state', () => {
    const m = parseAppManifest(`"AppState" { "appid" "570" "name" "Dota 2" "StateFlags" "4" "installdir" "dota 2 beta" "SizeOnDisk" "1000" "LastPlayed" "1700000000" }`)
    expect(m).toMatchObject({ appid: '570', name: 'Dota 2', installdir: 'dota 2 beta', sizeOnDisk: 1000, fullyInstalled: true, lastPlayed: 1700000000000 })
    expect(parseAppManifest(`"AppState" { "appid" "1" "name" "x" "StateFlags" "1026" }`)?.fullyInstalled).toBe(false)
    expect(parseAppManifest('garbage')).toBeNull()
  })

  it('reads play stats from localconfig', () => {
    const stats = parseLocalConfigApps(`"UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
      "620" { "LastPlayed" "1600000000" "Playtime" "95" }
      "cloud" { }
    } } } } }`)
    expect(stats.get('620')).toEqual({ lastPlayed: 1600000000000, playtime: 95 })
    expect(stats.has('cloud')).toBe(false)
  })
})

describe('registry', () => {
  const out = [
    '',
    'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1207658924',
    '    gameID    REG_SZ    1207658924',
    '    gameName    REG_SZ    Heroes of Might and Magic 3',
    '    path    REG_SZ    C:\\GOG Games\\HoMM 3 Complete',
    '    INSTALLDATE    REG_DWORD    0x10',
    '    (Default)    REG_SZ    ',
    '',
    'HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1207658924\\sub',
    '    x    REG_SZ    y',
    ''
  ].join('\r\n')

  it('parses reg query output', () => {
    const keys = parseRegQuery(out)
    expect(keys).toHaveLength(2)
    expect(keys[0].name).toBe('1207658924')
    expect(keys[0].values.gameName).toBe('Heroes of Might and Magic 3')
    expect(keys[0].values.path).toBe('C:\\GOG Games\\HoMM 3 Complete')
    expect(keys[0].values.INSTALLDATE).toBe('16')
    expect(keys[0].values['']).toBe('')
  })

  it('finds immediate children across hive spellings', () => {
    const kids = childKeys(parseRegQuery(out), 'HKLM\\SOFTWARE\\WOW6432Node\\GOG.com\\Games')
    expect(kids.map((k) => k.name)).toEqual(['1207658924'])
  })

  it('splits uninstall command lines', () => {
    expect(splitCommandLine('"C:\\Game Dir\\unins000.exe" /SILENT')).toEqual({ file: 'C:\\Game Dir\\unins000.exe', args: ['/SILENT'] })
    expect(splitCommandLine('C:\\Program Files\\Game\\uninstall.exe /x')).toEqual({ file: 'C:\\Program Files\\Game\\uninstall.exe', args: ['/x'] })
    expect(splitCommandLine('')).toBeNull()
  })
})

describe('other launcher parsers', () => {
  it('builds Epic app refs and filters DLC', () => {
    expect(epicAppRef('fn', 'abc', 'Fortnite')).toBe('fn%3Aabc%3AFortnite')
    expect(epicAppRef(undefined, undefined, 'Fortnite')).toBe('Fortnite')
    expect(isBaseGame({ AppName: 'a', DisplayName: 'A', MainGameAppName: 'b' })).toBe(false)
    expect(isBaseGame({ AppName: 'ue', DisplayName: 'UE', AppCategories: ['engines'] })).toBe(false)
    expect(isBaseGame({ AppName: 'a', DisplayName: 'A', MainGameAppName: 'a', AppCategories: ['public', 'games'] })).toBe(true)
  })

  it('decodes the Epic catalog cache and survives junk', () => {
    const raw = Buffer.from(JSON.stringify([{ id: 'x', namespace: 'n', title: 'T' }])).toString('base64')
    expect(parseCatalogCache(raw)[0].title).toBe('T')
    expect(parseCatalogCache('not base64 json')).toEqual([])
  })

  it('reads EA content ids', () => {
    const xml = '<DiPManifest><contentIDs><contentID>1234</contentID><contentID> 5678 </contentID></contentIDs></DiPManifest>'
    expect(parseContentIds(xml)).toEqual(['1234', '5678'])
  })

  it('reads Xbox game config and manifest', () => {
    expect(parseMicrosoftGameConfig('<Game><ShellVisuals DefaultDisplayName="Forza Horizon 5" /><StoreId>9NKX70BBCDRN</StoreId></Game>')).toEqual({ title: 'Forza Horizon 5', storeId: '9NKX70BBCDRN' })
    expect(parseMicrosoftGameConfig('<ShellVisuals DefaultDisplayName="ms-resource:Name"/>').title).toBeUndefined()
    expect(parseAppId('<Applications><Application Id="Game" Executable="x.exe">')).toBe('Game')
  })

  it('only opens launcher protocols', () => {
    expect(isAllowedUri('steam://rungameid/570')).toBe(true)
    expect(isAllowedUri('com.epicgames.launcher://apps/x?action=launch')).toBe(true)
    expect(isAllowedUri('file:///C:/Windows/system32/cmd.exe')).toBe(false)
    expect(isAllowedUri('javascript:alert(1)')).toBe(false)
  })
})
