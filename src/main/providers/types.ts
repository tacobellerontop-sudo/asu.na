import type { DetectedGame, LaunchAction, LauncherId } from '../../shared/types'
import type { RegistryReader } from '../platform/registry'

export interface KeyValueCache {
  get<T>(key: string): T | undefined
  set<T>(key: string, value: T): void
}

export interface ProviderContext {
  platform: NodeJS.Platform
  env: Record<string, string | undefined>
  home: string
  registry: RegistryReader
  /** Runs a PowerShell script and returns stdout ('' on failure or non-Windows). */
  powershell(script: string): Promise<string>
  /** GET a JSON document; rejects on network errors. */
  fetchJson(url: string): Promise<unknown>
  cache: KeyValueCache
  /** User-provided install folder for a launcher, when auto-detection fails. */
  pathOverride?: string
}

export interface Detection {
  detected: boolean
  /** Launcher install folder or client executable. */
  path?: string
}

export interface Provider {
  id: Exclude<LauncherId, 'custom'>
  name: string
  platforms: NodeJS.Platform[]
  downloadUrl: string
  detect(ctx: ProviderContext): Promise<Detection>
  scan(ctx: ProviderContext, detection: Detection): Promise<DetectedGame[]>
  /** How to bring the launcher itself to the front. */
  open(detection: Detection): LaunchAction
}
