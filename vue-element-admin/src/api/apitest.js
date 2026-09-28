import { invokeCommand } from '@/utils/tauri'

export function getSidebarTree() {
  return invokeCommand('get_sidebar_tree')
}

export function createCollection(payload) {
  return invokeCommand('create_collection', { input: payload })
}

export function renameCollection(payload) {
  return invokeCommand('rename_collection', { input: payload })
}

export function updateCollection(payload) {
  return invokeCommand('update_collection', { input: payload })
}

export function deleteCollection(id) {
  return invokeCommand('delete_collection', { id })
}

export function getRequest(id) {
  return invokeCommand('get_request', { id })
}

export function createRequest(payload) {
  return invokeCommand('create_request', { input: payload })
}

export function saveRequest(payload) {
  return invokeCommand('save_request', { input: payload })
}

export function deleteRequest(id) {
  return invokeCommand('delete_request', { id })
}

export function listEnvironments() {
  return invokeCommand('list_environments')
}

export function setActiveEnvironment(id) {
  return invokeCommand('set_active_environment', { id })
}

export function listEnvVars(environmentId) {
  return invokeCommand('list_env_vars', { environmentId })
}

export function upsertEnvVar(payload) {
  return invokeCommand('upsert_env_var', { input: payload })
}

export function deleteEnvVar(id) {
  return invokeCommand('delete_env_var', { id })
}

export function createEnvironment(name) {
  return invokeCommand('create_environment', { name })
}

export function sendRequest(payload) {
  return invokeCommand('send_request', { input: payload })
}

export function cancelRequest() {
  return invokeCommand('cancel_request')
}

export function listHistory(limit = 100) {
  return invokeCommand('list_history', { limit })
}

export function getHistory(id) {
  return invokeCommand('get_history', { id })
}

export function clearHistory() {
  return invokeCommand('clear_history')
}

export function deleteHistory(id) {
  return invokeCommand('delete_history', { id })
}

export function exportCollection(collectionId) {
  return invokeCommand('export_collection', { collectionId })
}

export function exportJmeter(payload) {
  return invokeCommand('export_jmeter', { input: payload })
}

export function runLoadTest(payload) {
  return invokeCommand('run_load_test_cmd', { input: payload })
}

export function importCollection(json) {
  return invokeCommand('import_collection', { input: { json: json }})
}

export function listWorkspaces() {
  return invokeCommand('list_workspaces')
}

export function createWorkspace(name) {
  return invokeCommand('create_workspace', { input: { name: name }})
}

export function setActiveWorkspace(id) {
  return invokeCommand('set_active_workspace', { id })
}

export function deleteWorkspace(id) {
  return invokeCommand('delete_workspace', { id })
}

export function getTheme() {
  return invokeCommand('get_theme')
}

export function setTheme(theme) {
  return invokeCommand('set_theme', { input: { theme: theme }})
}

export function getDbPath() {
  return invokeCommand('db_path')
}

export function getDbVersion() {
  return invokeCommand('db_version')
}

export function ping() {
  return invokeCommand('ping')
}
