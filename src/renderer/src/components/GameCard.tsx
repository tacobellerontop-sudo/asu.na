import type { Game } from '../../../shared/types'
import { LAUNCHER_META } from '../launchers'
import { Cover } from './Cover'

export function GameCard({ game, onOpen, onPrimary }: { game: Game; onOpen: () => void; onPrimary: () => void }) {
  const meta = LAUNCHER_META[game.launcher]
  return (
    <div className={`card ${game.installed ? '' : 'not-installed'}`} onClick={onOpen} role="button" tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && onOpen()}>
      <div className="cover">
        <Cover game={game} />
        {game.favorite && <span className="fav">★</span>}
        <div className="card-overlay">
          <button
            className={game.installed ? 'primary' : 'secondary'}
            onClick={(e) => {
              e.stopPropagation()
              onPrimary()
            }}
          >
            {game.installed ? '▶ Play' : '↓ Install'}
          </button>
        </div>
      </div>
      <div className="card-meta">
        <div className="card-title" title={game.title}>{game.title}</div>
        <div className="card-sub">
          <i className="dot" style={{ background: meta.color }} />
          {meta.short}
          {!game.installed && <span className="tag">Not installed</span>}
        </div>
      </div>
    </div>
  )
}
