import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import type { RegKey } from '../src/main/platform/registry'
import { normalizeKeyPath } from '../src/main/platform/registry'
import type { ProviderContext } from '../src/main/providers/types'

export function tempTree(files: Record<string, string>): string {
  const root = mkdtempSync(join(tmpdir(), 'asuna-'))
  for (const [rel, content] of Object.entries(files)) {
    const p = join(root, rel)
    mkdirSync(dirname(p), { recursive: true })
    writeFileSync(p, content)
  }
  return root
}

/** Fake registry: a query returns the key itself plus all subkeys, like `reg query /s`. */
export function fakeRegistry(keys: { path: string; values: Record<string, string> }[]) {
  const all: RegKey[] = keys.map((k) => ({ ...k, name: k.path.split('\\').pop()! }))
  return async (query: string) => {
    const q = normalizeKeyPath(query)
    return all.filter((k) => {
      const p = normalizeKeyPath(k.path)
      return p === q || p.startsWith(q + '\\')
    })
  }
}

export function makeCtx(over: Partial<ProviderContext> = {}): ProviderContext {
  const cache = new Map<string, unknown>()
  return {
    platform: 'win32',
    env: {},
    home: '/nonexistent-home',
    registry: async () => [],
    powershell: async () => '',
    fetchJson: async () => {
      throw new Error('offline')
    },
    cache: { get: (k) => cache.get(k) as never, set: (k, v) => void cache.set(k, v) },
    ...over
  }
}
