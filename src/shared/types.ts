export type LauncherId = 'steam' | 'epic' | 'gog' | 'battlenet' | 'ea' | 'ubisoft' | 'xbox' | 'custom'

/** Something the main process can do on the user's behalf. Never built from renderer input. */
export type LaunchAction =
  | { kind: 'uri'; uri: string }
  | { kind: 'exec'; file: string; args?: string[]; cwd?: string }

export type GameActionName = 'launch' | 'install' | 'uninstall' | 'store' | 'openFolder'

export interface GameArt {
  /** Portrait box art (roughly 2:3). */
  cover?: string
  /** Landscape header / banner. */
  header?: string
}

/** A game as reported by a provider, before user state is merged in. */
export interface DetectedGame {
  launcher: LauncherId
  /** Launcher-specific id (Steam appid, Epic AppName, GOG id, ...). */
  id: string
  title: string
  installed: boolean
  installPath?: string
  sizeBytes?: number
  /** Unix ms, when the launcher records it. */
  lastPlayed?: number
  playtimeMinutes?: number
  art?: GameArt
  /**
   * Listed from a launcher catalog without proof of ownership (e.g. Battle.net titles).
   * Shown under "Get games" rather than in the library until installed.
   */
  catalogOnly?: boolean
  actions: Partial<Record<Exclude<GameActionName, 'openFolder'>, LaunchAction>>
}

export interface Game extends DetectedGame {
  /** Unique across launchers: `${launcher}:${id}`. */
  key: string
  favorite: boolean
  hidden: boolean
}

export interface LauncherStatus {
  id: LauncherId
  name: string
  /** Whether this OS is supported by the provider at all. */
  supported: boolean
  enabled: boolean
  detected: boolean
  path?: string
  installedCount: number
  ownedCount: number
  downloadUrl?: string
  error?: string
}

export interface LibrarySnapshot {
  games: Game[]
  launchers: LauncherStatus[]
  scannedAt: number
  platform: string
}

export interface StoreResult {
  launcher: LauncherId
  id: string
  title: string
  image?: string
  price?: string
}

export interface CustomGameInput {
  title: string
  executable: string
  args?: string
}

export interface AsunaApi {
  getLibrary(): Promise<LibrarySnapshot>
  refresh(): Promise<LibrarySnapshot>
  runAction(gameKey: string, action: GameActionName): Promise<void>
  setFavorite(gameKey: string, favorite: boolean): Promise<void>
  setHidden(gameKey: string, hidden: boolean): Promise<void>
  setLauncherEnabled(id: LauncherId, enabled: boolean): Promise<LibrarySnapshot>
  openLauncher(id: LauncherId): Promise<void>
  getLauncher(id: LauncherId): Promise<void>
  searchStore(term: string): Promise<StoreResult[]>
  installFromStore(launcher: LauncherId, id: string): Promise<void>
  pickExecutable(): Promise<string | null>
  addCustomGame(input: CustomGameInput): Promise<LibrarySnapshot>
  removeCustomGame(gameKey: string): Promise<LibrarySnapshot>
}
