import type { LauncherId } from '../../shared/types'

export const LAUNCHER_META: Record<LauncherId, { label: string; short: string; color: string }> = {
  steam: { label: 'Steam', short: 'Steam', color: '#66c0f4' },
  epic: { label: 'Epic Games', short: 'Epic', color: '#e6e6e6' },
  gog: { label: 'GOG Galaxy', short: 'GOG', color: '#b36bff' },
  battlenet: { label: 'Battle.net', short: 'Battle.net', color: '#148eff' },
  ea: { label: 'EA app', short: 'EA', color: '#ff4747' },
  ubisoft: { label: 'Ubisoft Connect', short: 'Ubisoft', color: '#3d8bff' },
  xbox: { label: 'Xbox', short: 'Xbox', color: '#3fbf3f' },
  custom: { label: 'Added by you', short: 'Custom', color: '#f5a524' }
}

export function formatBytes(n?: number): string | undefined {
  if (!n) return undefined
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i++
  }
  return `${n.toFixed(n >= 10 || i === 0 ? 0 : 1)} ${units[i]}`
}

export function formatPlaytime(min?: number): string | undefined {
  if (!min) return undefined
  if (min < 60) return `${min} min`
  return `${(min / 60).toFixed(min < 600 ? 1 : 0)} h`
}

export function formatAgo(ts?: number): string | undefined {
  if (!ts) return undefined
  const d = Math.floor((Date.now() - ts) / 86_400_000)
  if (d <= 0) return 'Today'
  if (d === 1) return 'Yesterday'
  if (d < 30) return `${d} days ago`
  return new Date(ts).toLocaleDateString()
}
