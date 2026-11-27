import { useEffect } from 'react'
import type { Game, GameActionName } from '../../../shared/types'
import { LAUNCHER_META, formatAgo, formatBytes, formatPlaytime } from '../launchers'
import { Cover } from './Cover'

interface Props {
  game: Game
  onClose: () => void
  onAction: (a: GameActionName) => void
  onFavorite: () => void
  onHidden: () => void
  onRemove?: () => void
}

export function GameDetails({ game, onClose, onAction, onFavorite, onHidden, onRemove }: Props) {
  const meta = LAUNCHER_META[game.launcher]
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onClose()
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const facts: [string, string | undefined][] = [
    ['Status', game.installed ? 'Installed' : 'Not installed'],
    ['Size on disk', formatBytes(game.sizeBytes)],
    ['Play time', formatPlaytime(game.playtimeMinutes)],
    ['Last played', formatAgo(game.lastPlayed)],
    ['Folder', game.installPath]
  ]

  return (
    <div className="drawer-backdrop" onClick={onClose}>
      <aside className="drawer" onClick={(e) => e.stopPropagation()}>
        <button className="close" onClick={onClose} aria-label="Close">×</button>
        <div className="drawer-art">
          <Cover game={game} />
        </div>
        <div className="drawer-body">
          <div className="launcher-chip" style={{ ['--c' as string]: meta.color }}>{meta.label}</div>
          <h1>{game.title}</h1>

          <div className="drawer-actions">
            {game.installed ? (
              <button className="primary big" disabled={!game.actions.launch} onClick={() => onAction('launch')}>
                ▶ Play
              </button>
            ) : (
              <button className="primary big" disabled={!game.actions.install} onClick={() => onAction('install')}>
                ↓ Install with {meta.short}
              </button>
            )}
            <button className="icon" title={game.favorite ? 'Remove from favorites' : 'Add to favorites'} onClick={onFavorite}>
              {game.favorite ? '★' : '☆'}
            </button>
          </div>

          <dl className="facts">
            {facts
              .filter(([, v]) => v)
              .map(([k, v]) => (
                <div key={k}>
                  <dt>{k}</dt>
                  <dd title={v}>{v}</dd>
                </div>
              ))}
          </dl>

          <div className="drawer-links">
            {game.installed && game.installPath && <button onClick={() => onAction('openFolder')}>Open folder</button>}
            {game.actions.store && <button onClick={() => onAction('store')}>Open in {meta.short}</button>}
            {game.installed && game.actions.uninstall && <button onClick={() => onAction('uninstall')}>Uninstall</button>}
            <button onClick={onHidden}>{game.hidden ? 'Show in library' : 'Hide from library'}</button>
            {onRemove && <button className="danger" onClick={onRemove}>Remove from AsuNa</button>}
          </div>
        </div>
      </aside>
    </div>
  )
}
