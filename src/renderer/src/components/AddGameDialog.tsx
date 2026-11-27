import { useState } from 'react'
import type { AsunaApi, LibrarySnapshot } from '../../../shared/types'

export function AddGameDialog({ api, onClose, onAdded }: { api: AsunaApi; onClose: () => void; onAdded: (s: LibrarySnapshot) => void }) {
  const [title, setTitle] = useState('')
  const [exe, setExe] = useState('')
  const [args, setArgs] = useState('')
  const [error, setError] = useState('')

  const pick = async () => {
    const p = await api.pickExecutable()
    if (!p) return
    setExe(p)
    if (!title) setTitle(p.split(/[\\/]/).pop()!.replace(/\.(exe|bat|cmd|lnk|url|app)$/i, ''))
  }

  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      onAdded(await api.addCustomGame({ title, executable: exe, args }))
    } catch (err) {
      setError((err as Error).message)
    }
  }

  return (
    <div className="drawer-backdrop center" onClick={onClose}>
      <form className="dialog" onClick={(e) => e.stopPropagation()} onSubmit={submit}>
        <h2>Add a game</h2>
        <p className="muted">For games outside a supported launcher: emulators, itch.io downloads, DRM-free installs.</p>
        <label>
          Program
          <div className="row">
            <input value={exe} onChange={(e) => setExe(e.target.value)} placeholder="C:\Games\MyGame\game.exe" />
            <button type="button" onClick={pick}>Browse…</button>
          </div>
        </label>
        <label>
          Name
          <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="My Game" />
        </label>
        <label>
          Launch options <span className="muted">(optional)</span>
          <input value={args} onChange={(e) => setArgs(e.target.value)} placeholder="-windowed" />
        </label>
        {error && <div className="state bad">{error}</div>}
        <div className="row end">
          <button type="button" onClick={onClose}>Cancel</button>
          <button className="primary" disabled={!title.trim() || !exe.trim()}>Add game</button>
        </div>
      </form>
    </div>
  )
}
