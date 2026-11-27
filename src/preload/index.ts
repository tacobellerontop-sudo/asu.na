import { contextBridge, ipcRenderer } from 'electron'
import type { AsunaApi } from '../shared/types'

const api: AsunaApi = {
  getLibrary: () => ipcRenderer.invoke('library:get'),
  refresh: () => ipcRenderer.invoke('library:refresh'),
  runAction: (key, action) => ipcRenderer.invoke('game:action', key, action),
  setFavorite: (key, on) => ipcRenderer.invoke('game:favorite', key, on),
  setHidden: (key, on) => ipcRenderer.invoke('game:hidden', key, on),
  setLauncherEnabled: (id, on) => ipcRenderer.invoke('launcher:enabled', id, on),
  openLauncher: (id) => ipcRenderer.invoke('launcher:open', id),
  getLauncher: (id) => ipcRenderer.invoke('launcher:get', id),
  searchStore: (term) => ipcRenderer.invoke('store:search', term),
  installFromStore: (launcher, id) => ipcRenderer.invoke('store:install', launcher, id),
  pickExecutable: () => ipcRenderer.invoke('custom:pick'),
  addCustomGame: (input) => ipcRenderer.invoke('custom:add', input),
  removeCustomGame: (key) => ipcRenderer.invoke('custom:remove', key)
}

contextBridge.exposeInMainWorld('asuna', api)
