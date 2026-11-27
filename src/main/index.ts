import { join } from 'node:path'
import { BrowserWindow, app, dialog, ipcMain, shell } from 'electron'
import type { CustomGameInput, GameActionName, LauncherId, StoreResult } from '../shared/types'
import { runLaunchAction } from './actions'
import { Library } from './library'
import { BATTLENET_CATALOG } from './providers/battlenet'
import { PROVIDERS } from './providers'
import { steamActions } from './providers/steam'
import { Store } from './store'

const store = new Store(join(app.getPath('userData'), 'asuna-library.json'))
const library = new Library(store)

function createWindow(): void {
  const win = new BrowserWindow({
    width: 1280,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    backgroundColor: '#0e0f13',
    title: 'AsuNa Launcher',
    autoHideMenuBar: true,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false
    }
  })
  // Links in the UI open in the real browser, never inside the app window.
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https://')) void shell.openExternal(url)
    return { action: 'deny' }
  })
  if (process.env.ELECTRON_RENDERER_URL) void win.loadURL(process.env.ELECTRON_RENDERER_URL)
  else void win.loadFile(join(__dirname, '../renderer/index.html'))
}

async function snapshot() {
  return library.current ?? library.scan()
}

function requireGame(key: string) {
  const game = library.find(key)
  if (!game) throw new Error(`Unknown game: ${key}`)
  return game
}

function registerIpc(): void {
  ipcMain.handle('library:get', () => snapshot())
  ipcMain.handle('library:refresh', () => library.scan())

  ipcMain.handle('game:action', async (_e, key: string, action: GameActionName) => {
    const game = requireGame(key)
    if (action === 'openFolder') {
      if (!game.installPath) throw new Error('No install folder known for this game')
      const err = await shell.openPath(game.installPath)
      if (err) throw new Error(err)
      return
    }
    const a = game.actions[action]
    if (!a) throw new Error(`${game.title} cannot ${action} from AsuNa`)
    await runLaunchAction(a)
    if (action === 'launch') store.update((d) => (d.lastPlayed[key] = Date.now()))
  })

  ipcMain.handle('game:favorite', (_e, key: string, on: boolean) => {
    store.update((d) => {
      d.favorites = d.favorites.filter((k) => k !== key)
      if (on) d.favorites.push(key)
    })
    library.remerge()
  })

  ipcMain.handle('game:hidden', (_e, key: string, on: boolean) => {
    store.update((d) => {
      d.hidden = d.hidden.filter((k) => k !== key)
      if (on) d.hidden.push(key)
    })
    library.remerge()
  })

  ipcMain.handle('launcher:enabled', (_e, id: LauncherId, on: boolean) => {
    store.update((d) => {
      d.disabledLaunchers = d.disabledLaunchers.filter((l) => l !== id)
      if (!on) d.disabledLaunchers.push(id)
    })
    return library.scan()
  })

  ipcMain.handle('launcher:open', async (_e, id: LauncherId) => {
    const p = PROVIDERS.find((x) => x.id === id)
    const status = library.current?.launchers.find((l) => l.id === id)
    if (!p) throw new Error(`Unknown launcher: ${id}`)
    await runLaunchAction(p.open({ detected: !!status?.detected, path: status?.path }))
  })

  ipcMain.handle('launcher:get', async (_e, id: LauncherId) => {
    const p = PROVIDERS.find((x) => x.id === id)
    if (p) await shell.openExternal(p.downloadUrl)
  })

  ipcMain.handle('store:search', async (_e, term: string): Promise<StoreResult[]> => {
    const q = term.trim()
    if (!q) return []
    const results: StoreResult[] = BATTLENET_CATALOG.filter((c) =>
      c.title.toLowerCase().includes(q.toLowerCase())
    ).map((c) => ({ launcher: 'battlenet', id: c.code, title: c.title }))
    try {
      const res = await fetch(
        `https://store.steampowered.com/api/storesearch/?term=${encodeURIComponent(q)}&l=english&cc=US`,
        { signal: AbortSignal.timeout(10_000) }
      )
      const json = (await res.json()) as {
        items?: { id: number; name: string; tiny_image?: string; price?: { final: number; currency: string } }[]
      }
      for (const it of json.items ?? []) {
        results.push({
          launcher: 'steam',
          id: String(it.id),
          title: it.name,
          image: `https://cdn.cloudflare.steamstatic.com/steam/apps/${it.id}/header.jpg`,
          price: it.price ? `${(it.price.final / 100).toFixed(2)} ${it.price.currency}` : 'Free'
        })
      }
    } catch {
      // offline: Battle.net results only
    }
    return results
  })

  ipcMain.handle('store:install', async (_e, launcher: LauncherId, id: string) => {
    if (launcher === 'steam' && /^\d+$/.test(id)) return runLaunchAction(steamActions(id).install)
    if (launcher === 'battlenet' && BATTLENET_CATALOG.some((c) => c.code === id))
      return runLaunchAction({ kind: 'uri', uri: `battlenet://${id}` })
    throw new Error('Unsupported store item')
  })

  ipcMain.handle('custom:pick', async () => {
    const res = await dialog.showOpenDialog({
      title: 'Choose the game executable',
      properties: ['openFile'],
      filters:
        process.platform === 'win32'
          ? [{ name: 'Programs', extensions: ['exe', 'bat', 'cmd', 'lnk', 'url'] }, { name: 'All files', extensions: ['*'] }]
          : [{ name: 'All files', extensions: ['*'] }]
    })
    return res.canceled ? null : res.filePaths[0]
  })

  ipcMain.handle('custom:add', async (_e, input: CustomGameInput) => {
    if (!input?.title?.trim() || !input.executable?.trim()) throw new Error('Title and executable are required')
    store.update((d) =>
      d.customGames.push({
        id: `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`,
        title: input.title.trim(),
        executable: input.executable.trim(),
        args: input.args?.trim() || undefined
      })
    )
    await snapshot()
    return library.remerge()
  })

  ipcMain.handle('custom:remove', async (_e, key: string) => {
    const id = key.replace(/^custom:/, '')
    store.update((d) => (d.customGames = d.customGames.filter((c) => c.id !== id)))
    await snapshot()
    return library.remerge()
  })
}

app.whenReady().then(async () => {
  await store.load()
  registerIpc()
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})

app.on('before-quit', () => {
  void store.flush()
})
