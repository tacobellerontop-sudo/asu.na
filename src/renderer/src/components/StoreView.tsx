import { useMemo, useState } from 'react'
import type { AsunaApi, LibrarySnapshot, StoreResult } from '../../../shared/types'
import { LAUNCHER_META } from '../launchers'

interface Props {
  api: AsunaApi
  lib: LibrarySnapshot
  toast: (t: string, error?: boolean) => void
  onOpen: (key: string) => void
}

export function StoreView({ api, lib, toast, onOpen }: Props) {
  const [term, setTerm] = useState('')
  const [results, setResults] = useState<StoreResult[] | null>(null)
  const [busy, setBusy] = useState(false)

  const owned = useMemo(() => new Set(lib.games.map((g) => g.key)), [lib])
  const readyToInstall = lib.games.filter((g) => !g.installed && !g.catalogOnly && !g.hidden)
  const catalog = lib.games.filter((g) => g.catalogOnly && !g.installed)

  const search = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!term.trim()) return
    setBusy(true)
    try {
      setResults(await api.searchStore(term))
    } catch (err) {
      toast((err as Error).message, true)
    } finally {
      setBusy(false)
    }
  }

  const install = async (r: StoreResult) => {
    try {
      await api.installFromStore(r.launcher, r.id)
      toast(`Opening ${LAUNCHER_META[r.launcher].label} for ${r.title}`)
    } catch (err) {
      toast((err as Error).message, true)
    }
  }

  return (
    <section>
      <h2>Get games</h2>
      <form className="store-search" onSubmit={search}>
        <input className="search" placeholder="Search Steam and Battle.net" value={term} onChange={(e) => setTerm(e.target.value)} />
        <button className="primary" disabled={busy}>{busy ? 'Searching…' : 'Search'}</button>
      </form>

      {results && (
        <div className="store-results">
          {results.length === 0 && <div className="empty">No results. Store search needs an internet connection.</div>}
          {results.map((r) => {
            const key = `${r.launcher}:${r.id}`
            const have = owned.has(key) && !lib.games.find((g) => g.key === key)?.catalogOnly
            return (
              <div key={key} className="store-row">
                {r.image ? <img src={r.image} alt="" /> : <div className="store-img" style={{ ['--c' as string]: LAUNCHER_META[r.launcher].color }} />}
                <div className="store-info">
                  <div className="card-title">{r.title}</div>
                  <div className="card-sub">
                    <i className="dot" style={{ background: LAUNCHER_META[r.launcher].color }} />
                    {LAUNCHER_META[r.launcher].label}
                    {r.price && <span className="tag">{r.price}</span>}
                  </div>
                </div>
                {have ? (
                  <button onClick={() => onOpen(key)}>In library</button>
                ) : (
                  <button className="primary" onClick={() => install(r)}>Get</button>
                )}
              </div>
            )
          })}
        </div>
      )}

      <h2>Owned, not installed <span className="muted">· {readyToInstall.length}</span></h2>
      {readyToInstall.length === 0 ? (
        <div className="empty">Everything you own is installed, or AsuNa could not read the owned list.</div>
      ) : (
        <div className="chip-list">
          {readyToInstall.map((g) => (
            <button key={g.key} className="chip" onClick={() => onOpen(g.key)}>
              <i className="dot" style={{ background: LAUNCHER_META[g.launcher].color }} />
              {g.title}
            </button>
          ))}
        </div>
      )}

      {catalog.length > 0 && (
        <>
          <h2>Battle.net catalog</h2>
          <div className="chip-list">
            {catalog.map((g) => (
              <button key={g.key} className="chip" onClick={() => onOpen(g.key)}>
                <i className="dot" style={{ background: LAUNCHER_META[g.launcher].color }} />
                {g.title}
              </button>
            ))}
          </div>
        </>
      )}
    </section>
  )
}
