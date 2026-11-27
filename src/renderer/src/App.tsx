import { useCallback, useEffect, useMemo, useState } from 'react'
import type { AsunaApi, Game, GameActionName, LauncherId, LibrarySnapshot } from '../../shared/types'
import { mockApi } from './mockApi'
import { LAUNCHER_META } from './launchers'
import { GameCard } from './components/GameCard'
import { GameDetails } from './components/GameDetails'
import { LaunchersView } from './components/LaunchersView'
import { StoreView } from './components/StoreView'
import { AddGameDialog } from './components/AddGameDialog'

declare global {
  interface Window {
    asuna?: AsunaApi
  }
}

export const api: AsunaApi = window.asuna ?? mockApi

type View = 'library' | 'store' | 'launchers'
type Shelf = 'all' | 'installed' | 'notInstalled' | 'favorites' | 'recent' | 'hidden'
type Sort = 'title' | 'recent' | 'playtime' | 'size'

const SHELVES: { id: Shelf; label: string }[] = [
  { id: 'all', label: 'All games' },
  { id: 'installed', label: 'Installed' },
  { id: 'notInstalled', label: 'Ready to install' },
  { id: 'favorites', label: 'Favorites' },
  { id: 'recent', label: 'Recently played' },
  { id: 'hidden', label: 'Hidden' }
]

export interface Toast {
  id: number
  text: string
  error?: boolean
}

export function App() {
  const [lib, setLib] = useState<LibrarySnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [view, setView] = useState<View>('library')
  const [shelf, setShelf] = useState<Shelf>('all')
  const [launcher, setLauncher] = useState<LauncherId | 'any'>('any')
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState<Sort>('title')
  const [selected, setSelected] = useState<string | null>(null)
  const [adding, setAdding] = useState(false)
  const [toasts, setToasts] = useState<Toast[]>([])

  const toast = useCallback((text: string, error = false) => {
    const id = Date.now() + Math.random()
    setToasts((t) => [...t, { id, text, error }])
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), error ? 6000 : 3000)
  }, [])

  const load = useCallback(async (fresh: boolean) => {
    setLoading(true)
    try {
      setLib(await (fresh ? api.refresh() : api.getLibrary()))
    } catch (e) {
      toast(`Could not scan your library: ${(e as Error).message}`, true)
    } finally {
      setLoading(false)
    }
  }, [toast])

  useEffect(() => {
    void load(false)
  }, [load])

  const patchGame = (key: string, patch: Partial<Game>) =>
    setLib((l) => l && { ...l, games: l.games.map((g) => (g.key === key ? { ...g, ...patch } : g)) })

  const runAction = useCallback(
    async (game: Game, action: GameActionName) => {
      try {
        await api.runAction(game.key, action)
        const verb: Record<GameActionName, string> = {
          launch: `Launching ${game.title}`,
          install: `Opening ${LAUNCHER_META[game.launcher].label} to install ${game.title}`,
          uninstall: `Opening the uninstaller for ${game.title}`,
          store: `Opening the store page for ${game.title}`,
          openFolder: `Opening the folder for ${game.title}`
        }
        toast(verb[action])
        if (action === 'launch') patchGame(game.key, { lastPlayed: Date.now() })
      } catch (e) {
        toast((e as Error).message.replace(/^Error invoking remote method '[^']+': (Error: )?/, ''), true)
      }
    },
    [toast]
  )

  const toggleFavorite = async (g: Game) => {
    patchGame(g.key, { favorite: !g.favorite })
    await api.setFavorite(g.key, !g.favorite)
  }
  const toggleHidden = async (g: Game) => {
    patchGame(g.key, { hidden: !g.hidden })
    await api.setHidden(g.key, !g.hidden)
  }

  const library = useMemo(() => (lib?.games ?? []).filter((g) => !g.catalogOnly || g.installed), [lib])

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase()
    let list = library.filter((g) => {
      if (shelf === 'hidden' ? !g.hidden : g.hidden) return false
      if (launcher !== 'any' && g.launcher !== launcher) return false
      if (q && !g.title.toLowerCase().includes(q)) return false
      if (shelf === 'installed') return g.installed
      if (shelf === 'notInstalled') return !g.installed
      if (shelf === 'favorites') return g.favorite
      if (shelf === 'recent') return !!g.lastPlayed
      return true
    })
    const by: Record<Sort, (a: Game, b: Game) => number> = {
      title: (a, b) => a.title.localeCompare(b.title),
      recent: (a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0),
      playtime: (a, b) => (b.playtimeMinutes ?? 0) - (a.playtimeMinutes ?? 0),
      size: (a, b) => (b.sizeBytes ?? 0) - (a.sizeBytes ?? 0)
    }
    list = [...list].sort(shelf === 'recent' ? by.recent : by[sort])
    // Favorites float to the top of the default view.
    if (shelf === 'all' && sort === 'title') list.sort((a, b) => Number(b.favorite) - Number(a.favorite))
    return list
  }, [library, shelf, launcher, query, sort])

  const counts = useMemo(() => {
    const shown = library.filter((g) => !g.hidden)
    return {
      all: shown.length,
      installed: shown.filter((g) => g.installed).length,
      notInstalled: shown.filter((g) => !g.installed).length,
      favorites: shown.filter((g) => g.favorite).length,
      recent: shown.filter((g) => g.lastPlayed).length,
      hidden: library.filter((g) => g.hidden).length
    }
  }, [library])

  const perLauncher = useMemo(() => {
    const m = new Map<LauncherId, number>()
    for (const g of library) if (!g.hidden) m.set(g.launcher, (m.get(g.launcher) ?? 0) + 1)
    return m
  }, [library])

  const selectedGame = lib?.games.find((g) => g.key === selected) ?? null
  const continuePlaying = useMemo(
    () =>
      library
        .filter((g) => g.installed && g.lastPlayed && !g.hidden)
        .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))
        .slice(0, 4),
    [library]
  )

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="logo">A</div>
          <div>
            <div className="brand-name">AsuNa</div>
            <div className="brand-sub">Game manager</div>
          </div>
        </div>

        <nav className="nav">
          <button className={view === 'library' ? 'active' : ''} onClick={() => setView('library')}>Library</button>
          <button className={view === 'store' ? 'active' : ''} onClick={() => setView('store')}>Get games</button>
          <button className={view === 'launchers' ? 'active' : ''} onClick={() => setView('launchers')}>Launchers</button>
        </nav>

        {view === 'library' && (
          <>
            <div className="side-label">Shelves</div>
            <div className="side-list">
              {SHELVES.map((s) => (
                <button key={s.id} className={shelf === s.id ? 'active' : ''} onClick={() => setShelf(s.id)}>
                  <span>{s.label}</span>
                  <span className="count">{counts[s.id]}</span>
                </button>
              ))}
            </div>
            <div className="side-label">Launchers</div>
            <div className="side-list">
              <button className={launcher === 'any' ? 'active' : ''} onClick={() => setLauncher('any')}>
                <span>Every launcher</span>
              </button>
              {[...perLauncher.entries()].map(([id, n]) => (
                <button key={id} className={launcher === id ? 'active' : ''} onClick={() => setLauncher(id)}>
                  <span>
                    <i className="dot" style={{ background: LAUNCHER_META[id].color }} />
                    {LAUNCHER_META[id].label}
                  </span>
                  <span className="count">{n}</span>
                </button>
              ))}
            </div>
          </>
        )}

        <div className="side-foot">
          <button className="ghost" onClick={() => setAdding(true)}>+ Add a game</button>
          <button className="ghost" onClick={() => load(true)} disabled={loading}>
            {loading ? 'Scanning…' : 'Rescan launchers'}
          </button>
        </div>
      </aside>

      <main className="main">
        {view === 'library' && (
          <>
            <header className="toolbar">
              <input
                className="search"
                placeholder="Search your games"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <select value={sort} onChange={(e) => setSort(e.target.value as Sort)} disabled={shelf === 'recent'}>
                <option value="title">A to Z</option>
                <option value="recent">Recently played</option>
                <option value="playtime">Most played</option>
                <option value="size">Largest on disk</option>
              </select>
            </header>

            {shelf === 'all' && launcher === 'any' && !query && continuePlaying.length > 0 && (
              <section className="hero-row">
                <h2>Jump back in</h2>
                <div className="hero-grid">
                  {continuePlaying.map((g) => (
                    <button key={g.key} className="hero" onClick={() => setSelected(g.key)}>
                      <HeroImage game={g} />
                      <div className="hero-info">
                        <span>{g.title}</span>
                        <span className="play-pill" onClick={(e) => (e.stopPropagation(), void runAction(g, 'launch'))}>
                          ▶ Play
                        </span>
                      </div>
                    </button>
                  ))}
                </div>
              </section>
            )}

            <section>
              <h2>
                {SHELVES.find((s) => s.id === shelf)?.label}
                {launcher !== 'any' && <span className="muted"> · {LAUNCHER_META[launcher].label}</span>}
              </h2>
              {loading && !lib ? (
                <div className="empty">Looking for games across your launchers…</div>
              ) : visible.length === 0 ? (
                <div className="empty">
                  {library.length === 0
                    ? 'No games found yet. Check the Launchers page, or add a game yourself.'
                    : 'Nothing matches this filter.'}
                </div>
              ) : (
                <div className="grid">
                  {visible.map((g) => (
                    <GameCard
                      key={g.key}
                      game={g}
                      onOpen={() => setSelected(g.key)}
                      onPrimary={() => runAction(g, g.installed ? 'launch' : 'install')}
                    />
                  ))}
                </div>
              )}
            </section>
          </>
        )}

        {view === 'store' && lib && <StoreView api={api} lib={lib} toast={toast} onOpen={(k) => setSelected(k)} />}
        {view === 'launchers' && lib && (
          <LaunchersView
            api={api}
            lib={lib}
            onChange={setLib}
            toast={toast}
            onShowGames={(id) => {
              setLauncher(id)
              setShelf('all')
              setView('library')
            }}
          />
        )}
      </main>

      {selectedGame && (
        <GameDetails
          game={selectedGame}
          onClose={() => setSelected(null)}
          onAction={(a) => runAction(selectedGame, a)}
          onFavorite={() => toggleFavorite(selectedGame)}
          onHidden={() => toggleHidden(selectedGame)}
          onRemove={
            selectedGame.launcher === 'custom'
              ? async () => {
                  setLib(await api.removeCustomGame(selectedGame.key))
                  setSelected(null)
                }
              : undefined
          }
        />
      )}

      {adding && (
        <AddGameDialog
          api={api}
          onClose={() => setAdding(false)}
          onAdded={(s) => {
            setLib(s)
            setAdding(false)
            toast('Game added to your library')
          }}
        />
      )}

      <div className="toasts">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.error ? 'error' : ''}`}>{t.text}</div>
        ))}
      </div>
    </div>
  )
}

function HeroImage({ game }: { game: Game }) {
  const [failed, setFailed] = useState(false)
  if (!game.art?.header || failed) return <div className="hero-fallback" style={{ ['--c' as string]: LAUNCHER_META[game.launcher].color }} />
  return <img src={game.art.header} alt="" onError={() => setFailed(true)} />
}
