import { spawn } from 'node:child_process'
import { shell } from 'electron'
import type { LaunchAction } from '../shared/types'

/** Only launcher protocols (plus the web store pages) may be opened from the main process. */
const ALLOWED_SCHEMES = new Set([
  'steam:',
  'com.epicgames.launcher:',
  'goggalaxy:',
  'battlenet:',
  'origin2:',
  'link2ea:',
  'uplay:',
  'xbox:',
  'ms-windows-store:',
  'ms-settings:',
  'https:'
])

export function isAllowedUri(uri: string): boolean {
  try {
    return ALLOWED_SCHEMES.has(new URL(uri).protocol)
  } catch {
    return false
  }
}

export async function runLaunchAction(action: LaunchAction): Promise<void> {
  if (action.kind === 'uri') {
    if (!isAllowedUri(action.uri)) throw new Error(`Blocked link: ${action.uri}`)
    await shell.openExternal(action.uri)
    return
  }
  await new Promise<void>((resolve, reject) => {
    const child = spawn(action.file, action.args ?? [], {
      cwd: action.cwd,
      detached: true,
      stdio: 'ignore',
      windowsHide: false
    })
    child.once('error', reject)
    child.once('spawn', () => {
      child.unref()
      resolve()
    })
  })
}
