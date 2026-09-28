/**
 * Lightweight Postman-like script sandbox for ApiTest.
 * API surface:
 *   at.env.get/set/unset
 *   at.request  (mutable: url, method, headers, params, body_type, body_content)
 *   at.response (post only): status, body, headers, json()
 *   at.test(name, fn)
 *   at.expect(actual).toBe / toEqual / toContain / toBeTruthy / toBeFalsy
 */

function makeExpect(actual) {
  return {
    toBe(expected) {
      if (actual !== expected) {
        throw new Error(`expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`)
      }
    },
    toEqual(expected) {
      const a = JSON.stringify(actual)
      const b = JSON.stringify(expected)
      if (a !== b) throw new Error(`expected ${b}, got ${a}`)
    },
    toContain(part) {
      const s = typeof actual === 'string' ? actual : JSON.stringify(actual)
      if (!s.includes(String(part))) {
        throw new Error(`expected to contain ${JSON.stringify(part)}`)
      }
    },
    toBeTruthy() {
      if (!actual) throw new Error('expected truthy value')
    },
    toBeFalsy() {
      if (actual) throw new Error('expected falsy value')
    }
  }
}

function createAtContext({ envMap, request, response }) {
  const envUpdates = {}
  const tests = []

  const at = {
    env: {
      get(key) {
        if (Object.prototype.hasOwnProperty.call(envUpdates, key)) return envUpdates[key]
        return envMap[key]
      },
      set(key, value) {
        envUpdates[String(key)] = value == null ? '' : String(value)
      },
      unset(key) {
        envUpdates[String(key)] = null
      }
    },
    request,
    response: response || null,
    test(name, fn) {
      try {
        fn()
        tests.push({ name: String(name), passed: true, error: null })
      } catch (e) {
        tests.push({
          name: String(name),
          passed: false,
          error: e && e.message ? e.message : String(e)
        })
      }
    },
    expect(actual) {
      return makeExpect(actual)
    }
  }

  if (response) {
    at.response = {
      status: response.status,
      status_text: response.status_text,
      body: response.body,
      headers: response.headers || [],
      url: response.url,
      mocked: !!response.mocked,
      json() {
        try {
          return JSON.parse(response.body || 'null')
        } catch (e) {
          throw new Error('response body is not valid JSON')
        }
      }
    }
  }

  return { at, envUpdates, tests }
}

export function runPreScript(code, { envMap, request }) {
  const req = {
    method: request.method,
    url: request.url,
    headers: (request.headers || []).map(h => ({ ...h })),
    params: (request.params || []).map(p => ({ ...p })),
    body_type: request.body_type,
    body_content: request.body_content
  }
  const { at, envUpdates } = createAtContext({ envMap, request: req, response: null })
  if (code && code.trim()) {
    // eslint-disable-next-line no-new-func
    const fn = new Function('at', code)
    fn(at)
  }
  return { request: req, envUpdates }
}

export function runTestScript(code, { envMap, request, response }) {
  const { at, envUpdates, tests } = createAtContext({
    envMap,
    request: {
      method: request.method,
      url: request.url,
      headers: request.headers,
      params: request.params,
      body_type: request.body_type,
      body_content: request.body_content
    },
    response
  })
  if (code && code.trim()) {
    // eslint-disable-next-line no-new-func
    const fn = new Function('at', code)
    fn(at)
  }
  return { tests, envUpdates }
}
