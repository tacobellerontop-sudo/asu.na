import { promises as fs } from 'node:fs'
import { dirname } from 'node:path'
import type { LauncherId } from '../shared/types'
import type { KeyValueCache } from './providers/types'

export interface CustomGame {
  id: string
  title: string
  executable: string
  args?: string
}

export interface StoreData {
  favorites: string[]
  hidden: string[]
  /** Last launch through AsuNa, keyed by game key. */
  lastPlayed: Record<string, number>
  disabledLaunchers: LauncherId[]
  pathOverrides: Partial<Record<LauncherId, string>>
  customGames: CustomGame[]
  cache: Record<string, unknown>
}

const EMPTY: StoreData = {
  favorites: [],
  hidden: [],
  lastPlayed: {},
  disabledLaunchers: [],
  pathOverrides: {},
  customGames: [],
  cache: {}
}

/** Small JSON file store. Writes are debounced and atomic (write temp, then rename). */
export class Store implements KeyValueCache {
  data: StoreData = structuredClone(EMPTY)
  private timer: NodeJS.Timeout | null = null

  constructor(private file: string) {}

  async load(): Promise<void> {
    try {
      const parsed = JSON.parse(await fs.readFile(this.file, 'utf8'))
      this.data = { ...structuredClone(EMPTY), ...parsed }
    } catch {
      this.data = structuredClone(EMPTY)
    }
  }

  update(fn: (d: StoreData) => void): void {
    fn(this.data)
    this.scheduleSave()
  }

  get<T>(key: string): T | undefined {
    return this.data.cache[key] as T | undefined
  }

  set<T>(key: string, value: T): void {
    this.data.cache[key] = value
    this.scheduleSave()
  }

  private scheduleSave(): void {
    if (this.timer) clearTimeout(this.timer)
    this.timer = setTimeout(() => void this.flush(), 300)
  }

  async flush(): Promise<void> {
    if (this.timer) clearTimeout(this.timer)
    this.timer = null
    await fs.mkdir(dirname(this.file), { recursive: true })
    const tmp = `${this.file}.tmp`
    await fs.writeFile(tmp, JSON.stringify(this.data, null, 2))
    await fs.rename(tmp, this.file)
  }
}
