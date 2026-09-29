/** Common JSON field names that usually hold an auth token. */
const TOKEN_KEY_RE = /^(access[_-]?token|id[_-]?token|refresh[_-]?token|auth[_-]?token|bearer|jwt|token)$/i

/**
 * Walk JSON and collect string values whose key looks like a token.
 * @returns {{ path: string, key: string, value: string }[]}
 */
export function findTokenCandidates(data, max = 20) {
  const out = []
  const seen = new Set()

  function push(path, key, value) {
    if (typeof value !== 'string') return
    const v = value.trim()
    if (!v || v.length < 4) return
    if (seen.has(v)) return
    seen.add(v)
    out.push({ path, key, value: v })
  }

  function walk(node, path) {
    if (out.length >= max || node == null) return
    if (Array.isArray(node)) {
      node.forEach((item, i) => walk(item, path ? `${path}[${i}]` : `[${i}]`))
      return
    }
    if (typeof node !== 'object') return
    Object.keys(node).forEach(key => {
      const child = node[key]
      const childPath = path ? `${path}.${key}` : key
      if (TOKEN_KEY_RE.test(key) && typeof child === 'string') {
        push(childPath, key, child)
      } else if (child && typeof child === 'object') {
        walk(child, childPath)
      }
    })
  }

  walk(data, '')
  // Prefer shorter paths / more specific keys
  out.sort((a, b) => {
    const score = (c) => {
      const k = c.key.toLowerCase()
      if (k === 'access_token' || k === 'accesstoken') return 0
      if (k === 'token') return 1
      if (k.includes('access')) return 2
      return 3
    }
    return score(a) - score(b) || a.path.length - b.path.length
  })
  return out
}

export function parseResponseJson(body) {
  if (!body || !String(body).trim()) return null
  try {
    return JSON.parse(body)
  } catch (e) {
    return null
  }
}

/** Upsert Authorization header; preserve other headers. */
export function upsertAuthHeader(headers, value = 'Bearer {{token}}') {
  const list = Array.isArray(headers)
    ? headers.map(h => ({ ...h }))
    : []
  const idx = list.findIndex(h => h.key && String(h.key).toLowerCase() === 'authorization')
  if (idx >= 0) {
    list[idx] = {
      ...list[idx],
      key: 'Authorization',
      value,
      enabled: true
    }
  } else {
    // Insert after any empty trailing row, or at end before blank row
    const blankIdx = list.findIndex(h => !(h.key || '').trim() && !(h.value || '').trim())
    const row = { key: 'Authorization', value, enabled: true }
    if (blankIdx >= 0) {
      list.splice(blankIdx, 0, row)
    } else {
      list.push(row)
    }
  }
  return list
}

/** Collect request ids under a collection node (recursive). */
export function collectRequestIds(treeNodes, collectionId, excludeId) {
  const ids = []
  function walk(nodes) {
    ;(nodes || []).forEach(n => {
      if (!n) return
      if (n.node_type === 'request' && n.collection_id === collectionId && n.request_id != null) {
        if (excludeId == null || n.request_id !== excludeId) {
          ids.push(n.request_id)
        }
      }
      if (n.children && n.children.length) walk(n.children)
    })
  }
  walk(treeNodes)
  return ids
}
