import { execFile } from 'node:child_process'

/** One registry key with its string-ish values (REG_SZ, REG_EXPAND_SZ, REG_DWORD as decimal string). */
export interface RegKey {
  path: string
  /** Last path segment, e.g. the GOG game id. */
  name: string
  values: Record<string, string>
}

/**
 * Parses the text output of `reg query <key> /s`.
 * Key lines start at column 0 with HKEY_..., values are indented by 4 spaces:
 *     name    REG_SZ    value
 */
export function parseRegQuery(output: string): RegKey[] {
  const keys: RegKey[] = []
  let cur: RegKey | null = null
  for (const raw of output.split(/\r?\n/)) {
    if (!raw.trim()) continue
    if (/^HKEY_/i.test(raw)) {
      const path = raw.trim()
      cur = { path, name: path.split('\\').pop() ?? path, values: {} }
      keys.push(cur)
      continue
    }
    const m = /^\s+(.*?)\s{4}(REG_[A-Z_]+)\s{4}?(.*)$/.exec(raw)
    if (m && cur) {
      const [, rawName, type, rawValue] = m
      const name = rawName === '(Default)' || rawName === '(既定)' ? '' : rawName
      let value = rawValue ?? ''
      if (type === 'REG_DWORD' || type === 'REG_QWORD') {
        const num = Number.parseInt(value, 16)
        value = Number.isNaN(num) ? value : String(num)
      }
      cur.values[name] = value
    }
  }
  return keys
}

export type RegistryReader = (key: string) => Promise<RegKey[]>

/** Reads a key recursively with reg.exe. Returns [] when the key is missing or not on Windows. */
export const readRegistry: RegistryReader = (key) =>
  new Promise((resolve) => {
    if (process.platform !== 'win32') return resolve([])
    execFile(
      'reg',
      ['query', key, '/s'],
      { windowsHide: true, maxBuffer: 32 * 1024 * 1024 },
      (err, stdout) => resolve(err ? [] : parseRegQuery(stdout))
    )
  })

/** Value lookup that ignores case on the value name. */
export function regValue(key: RegKey | undefined, name: string): string | undefined {
  if (!key) return undefined
  const lower = name.toLowerCase()
  const k = Object.keys(key.values).find((v) => v.toLowerCase() === lower)
  return k === undefined ? undefined : key.values[k]
}

/** Immediate children of `parent` in a recursive query result. */
export function childKeys(all: RegKey[], parent: string): RegKey[] {
  const prefix = normalizeKeyPath(parent).replace(/\\+$/, '') + '\\'
  return all.filter((k) => {
    const p = normalizeKeyPath(k.path)
    return p.startsWith(prefix) && !p.slice(prefix.length).includes('\\')
  })
}

export const UNINSTALL_KEYS = [
  'HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall',
  'HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall',
  'HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall'
]

const HIVES: Record<string, string> = {
  hklm: 'hkey_local_machine',
  hkcu: 'hkey_current_user',
  hkcr: 'hkey_classes_root',
  hku: 'hkey_users'
}

/** Lower-cases a key path and expands short hive names so paths can be compared. */
export function normalizeKeyPath(path: string): string {
  const lower = path.toLowerCase()
  const [hive, ...rest] = lower.split('\\')
  return [HIVES[hive] ?? hive, ...rest].join('\\')
}

/**
 * Splits a Windows command line such as `"C:\\Game\\unins000.exe" /SILENT` into file + args.
 * Good enough for UninstallString values; not a full CommandLineToArgvW implementation.
 */
export function splitCommandLine(cmd: string): { file: string; args: string[] } | null {
  const parts: string[] = []
  const re = /"([^"]*)"|(\S+)/g
  let m: RegExpExecArray | null
  while ((m = re.exec(cmd.trim()))) parts.push(m[1] ?? m[2])
  if (!parts.length) return null
  // Unquoted paths with spaces: grow the file part until it ends in .exe
  if (!/^"/.test(cmd.trim()) && !/\.exe$/i.test(parts[0])) {
    const idx = parts.findIndex((p) => /\.exe$/i.test(p))
    if (idx > 0) return { file: parts.slice(0, idx + 1).join(' '), args: parts.slice(idx + 1) }
  }
  return { file: parts[0], args: parts.slice(1) }
}
