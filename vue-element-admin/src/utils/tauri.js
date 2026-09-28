/**
 * Tauri bridge helpers.
 * Prefer withGlobalTauri inject to avoid Vue CLI 4 / webpack ESM friction.
 */

export function isTauriRuntime() {
  return !!(typeof window !== 'undefined' && (window.__TAURI_INTERNALS__ || window.__TAURI__))
}

export async function invokeCommand(cmd, args = {}) {
  if (!isTauriRuntime()) {
    throw new Error('当前不在 Tauri 运行时，请使用根目录命令：npm run tauri:dev')
  }

  if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
    return window.__TAURI__.core.invoke(cmd, args)
  }

  if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
    return window.__TAURI_INTERNALS__.invoke(cmd, args)
  }

  const { invoke } = await import('@tauri-apps/api/core')
  return invoke(cmd, args)
}

async function getAppWindow() {
  if (!isTauriRuntime()) return null
  if (window.__TAURI__ && window.__TAURI__.window && typeof window.__TAURI__.window.getCurrentWindow === 'function') {
    return window.__TAURI__.window.getCurrentWindow()
  }
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  return getCurrentWindow()
}

export async function windowMinimize() {
  const win = await getAppWindow()
  if (win) await win.minimize()
}

export async function windowToggleMaximize() {
  const win = await getAppWindow()
  if (win) await win.toggleMaximize()
}

export async function windowClose() {
  const win = await getAppWindow()
  if (win) await win.close()
}

export async function windowIsMaximized() {
  const win = await getAppWindow()
  if (!win) return false
  return win.isMaximized()
}

export async function windowStartDragging() {
  const win = await getAppWindow()
  if (win) await win.startDragging()
}
