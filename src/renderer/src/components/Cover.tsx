import { useState } from 'react'
import type { Game } from '../../../shared/types'
import { LAUNCHER_META } from '../launchers'

/** Box art with a fallback chain: cover → header → generated tile. */
export function Cover({ game }: { game: Game }) {
  const sources = [game.art?.cover, game.art?.header].filter(Boolean) as string[]
  const [idx, setIdx] = useState(0)
  const src = sources[idx]
  if (src) {
    return (
      <img
        className={`cover-img ${idx > 0 ? 'wide' : ''}`}
        src={src}
        alt=""
        loading="lazy"
        onError={() => setIdx((i) => i + 1)}
      />
    )
  }
  const initials = game.title
    .replace(/[^\p{L}\p{N}\s]/gu, '')
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0])
    .join('')
    .toUpperCase()
  return (
    <div className="cover-fallback" style={{ ['--c' as string]: LAUNCHER_META[game.launcher].color }}>
      <span className="initials">{initials}</span>
      <span className="fallback-title">{game.title}</span>
    </div>
  )
}
