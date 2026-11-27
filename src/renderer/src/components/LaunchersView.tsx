import type { AsunaApi, LauncherId, LibrarySnapshot } from '../../../shared/types'
import { LAUNCHER_META } from '../launchers'

interface Props {
  api: AsunaApi
  lib: LibrarySnapshot
  onChange: (s: LibrarySnapshot) => void
  toast: (t: string, error?: boolean) => void
  onShowGames: (id: LauncherId) => void
}

export function LaunchersView({ api, lib, onChange, toast, onShowGames }: Props) {
  const wrap = (fn: () => Promise<unknown>) => async () => {
    try {
      await fn()
    } catch (e) {
      toast((e as Error).message, true)
    }
  }
  return (
    <section>
      <h2>Launchers</h2>
      <p className="muted lead">
        AsuNa reads each launcher&apos;s local files to build one library, then hands launches and installs back to that
        launcher. Turn a launcher off to leave it out of scans.
      </p>
      <div className="launcher-grid">
        {lib.launchers
          .filter((l) => l.id !== 'custom')
          .map((l) => {
            const meta = LAUNCHER_META[l.id]
            const state = !l.supported
              ? { text: `Not available on this OS`, cls: 'off' }
              : !l.enabled
                ? { text: 'Turned off', cls: 'off' }
                : l.error
                  ? { text: 'Scan failed', cls: 'bad' }
                  : l.detected
                    ? { text: 'Connected', cls: 'ok' }
                    : { text: 'Not found', cls: 'warn' }
            return (
              <div key={l.id} className="launcher-card" style={{ ['--c' as string]: meta.color }}>
                <div className="launcher-head">
                  <div className="launcher-mark">{meta.short[0]}</div>
                  <div>
                    <div className="launcher-name">{l.name}</div>
                    <div className={`state ${state.cls}`}>{state.text}</div>
                  </div>
                  {l.supported && (
                    <label className="switch" title={l.enabled ? 'Turn off' : 'Turn on'}>
                      <input
                        type="checkbox"
                        checked={l.enabled}
                        onChange={wrap(async () => onChange(await api.setLauncherEnabled(l.id, !l.enabled)))}
                      />
                      <span />
                    </label>
                  )}
                </div>
                {l.detected && l.enabled && (
                  <div className="launcher-stats">
                    <div><b>{l.installedCount}</b> installed</div>
                    <div><b>{l.ownedCount - l.installedCount}</b> ready to install</div>
                  </div>
                )}
                {l.path && <div className="path" title={l.path}>{l.path}</div>}
                {l.error && <div className="path bad">{l.error}</div>}
                <div className="launcher-actions">
                  {l.detected && l.enabled ? (
                    <>
                      <button onClick={() => onShowGames(l.id)}>Show games</button>
                      <button onClick={wrap(() => api.openLauncher(l.id))}>Open {meta.short}</button>
                    </>
                  ) : (
                    l.supported && <button onClick={wrap(() => api.getLauncher(l.id))}>Get {meta.short}</button>
                  )}
                </div>
              </div>
            )
          })}
      </div>
      <p className="muted small">Last scan {new Date(lib.scannedAt).toLocaleTimeString()}</p>
    </section>
  )
}
