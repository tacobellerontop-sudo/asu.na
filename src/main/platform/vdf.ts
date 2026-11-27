/**
 * Minimal parser for Valve's text KeyValues format (.vdf / .acf).
 * Keys are kept as written; use `getCI` for case-insensitive lookups
 * because Steam is inconsistent about casing ("apps" vs "Apps").
 */
export type VdfValue = string | VdfObject
export interface VdfObject {
  [key: string]: VdfValue
}

export function parseVdf(text: string): VdfObject {
  let i = 0
  const n = text.length

  function skip(): void {
    while (i < n) {
      const c = text[i]
      if (c === ' ' || c === '\t' || c === '\r' || c === '\n' || c === '﻿') {
        i++
      } else if (c === '/' && text[i + 1] === '/') {
        while (i < n && text[i] !== '\n') i++
      } else if (c === '[') {
        // conditional like [$WIN32], ignore
        while (i < n && text[i] !== ']') i++
        i++
      } else {
        break
      }
    }
  }

  function token(): string | null {
    skip()
    if (i >= n) return null
    const c = text[i]
    if (c === '{' || c === '}') {
      i++
      return c
    }
    if (c === '"') {
      i++
      let out = ''
      while (i < n && text[i] !== '"') {
        if (text[i] === '\\' && i + 1 < n) {
          const e = text[i + 1]
          out += e === 'n' ? '\n' : e === 't' ? '\t' : e
          i += 2
        } else {
          out += text[i++]
        }
      }
      i++
      return out
    }
    let out = ''
    while (i < n && !/[\s{}"]/.test(text[i])) out += text[i++]
    return out
  }

  function parseObject(): VdfObject {
    const obj: VdfObject = {}
    for (;;) {
      const key = token()
      if (key === null || key === '}') return obj
      const save = i
      const next = token()
      if (next === '{') {
        obj[key] = parseObject()
      } else if (next === null || next === '}') {
        obj[key] = ''
        if (next === '}') return obj
        i = save
      } else {
        obj[key] = next
      }
    }
  }

  return parseObject()
}

export function getCI(obj: VdfValue | undefined, ...path: string[]): VdfValue | undefined {
  let cur: VdfValue | undefined = obj
  for (const p of path) {
    if (!cur || typeof cur === 'string') return undefined
    const lower = p.toLowerCase()
    const k: string | undefined = Object.keys(cur).find((key) => key.toLowerCase() === lower)
    cur = k === undefined ? undefined : cur[k]
  }
  return cur
}

export function asObject(v: VdfValue | undefined): VdfObject | undefined {
  return v && typeof v === 'object' ? v : undefined
}

export function asString(v: VdfValue | undefined): string | undefined {
  return typeof v === 'string' ? v : undefined
}
