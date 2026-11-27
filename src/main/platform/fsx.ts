import { promises as fs } from 'node:fs'

export async function exists(p: string | undefined): Promise<boolean> {
  if (!p) return false
  try {
    await fs.access(p)
    return true
  } catch {
    return false
  }
}

export async function readText(p: string): Promise<string | null> {
  try {
    return await fs.readFile(p, 'utf8')
  } catch {
    return null
  }
}

export async function listDir(p: string): Promise<string[]> {
  try {
    return await fs.readdir(p)
  } catch {
    return []
  }
}

export async function firstExisting(paths: (string | undefined)[]): Promise<string | undefined> {
  for (const p of paths) if (await exists(p)) return p
  return undefined
}

/** Runs `fn` over items with at most `limit` in flight. */
export async function mapLimit<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const out: R[] = new Array(items.length)
  let next = 0
  const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (next < items.length) {
      const idx = next++
      out[idx] = await fn(items[idx])
    }
  })
  await Promise.all(workers)
  return out
}
