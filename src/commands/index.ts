import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import EventStore from '../state/event-store'
import type Queries from '../queries'
import { ensureInitialCommit, commitZone, commitDelete, readBlobAtCommit } from '../git/service'

export default class Commands {
  state: EventStore
  queries: Queries
  getPort: () => number
  getIdentity: () => string
  getIdentityName: () => string

  constructor({ state, queries, getPort, getIdentity, getIdentityName }: { state: EventStore; queries: Queries; getPort: () => number; getIdentity: () => string; getIdentityName: () => string }) {
    this.state = state
    this.queries = queries
    this.getPort = getPort
    this.getIdentity = getIdentity
    this.getIdentityName = getIdentityName
  }

  async loadZones() {
    const identity = this.getIdentity()
    const zones = await invoke<Array<{ name: string; content: string }>>('list_zones', { identity })

    const existing = new Set(this.queries.allZones().map((z: any) => z.name))
    const incoming = new Set(zones.map(z => z.name))

    for (const zone of zones) {
      this.state.track('zones', zone.name, existing.has(zone.name) ? 'update' : 'create', {
        name: zone.name,
        content: zone.content,
      })
    }

    for (const name of existing) {
      if (!incoming.has(name)) {
        this.state.track('zones', name, 'delete')
      }
    }

    try { await ensureInitialCommit(identity, zones) } catch (e) { console.error('git init failed:', e) }
  }

  async addZone(name: string) {
    const identity = this.getIdentity()
    await invoke('create_zone', { identity, name, port: this.getPort() })
    await invoke('reload_server', { identity })
    const content = await invoke<string>('read_zone', { identity, name })
    const event = this.state.track('zones', name, 'create', { name, content })
    try { await commitZone(identity, name, 'Create', content) } catch (e) { console.error('git commit failed:', e) }
    emit('zones-changed-' + this.getIdentity())
    return this.queries.findZone(event.objectId)
  }

  async updateZone(zone: { id: string }, content: string, message?: string) {
    const identity = this.getIdentity()
    const saved = await invoke<string>('save_zone', { identity, name: zone.id, content, port: this.getPort() })
    await invoke('reload_server', { identity })
    try { await commitZone(identity, zone.id, message || `Update ${zone.id}`, saved) } catch (e) { console.error('git commit failed:', e) }
    this.state.track('zones', zone.id, 'update', { content: saved })
    emit('zones-changed-' + this.getIdentity())
  }

  async removeZone(zone: { id: string }) {
    const identity = this.getIdentity()
    await invoke('delete_zone', { identity, name: zone.id, port: this.getPort() })
    await invoke('reload_server', { identity })
    this.state.track('zones', zone.id, 'delete')
    try { await commitDelete(identity, zone.id) } catch (e) { console.error('git commit failed:', e) }
    emit('zones-changed-' + this.getIdentity())
  }

  async readZone(name: string): Promise<string> {
    return invoke('read_zone', { identity: this.getIdentity(), name })
  }

  async startServer() {
    await invoke('start_server', { identity: this.getIdentity(), port: this.getPort() })
  }

  async stopServer() {
    await invoke('stop_server', { identity: this.getIdentity() })
  }

  async stopAllServers() {
    await invoke('stop_all_servers')
  }

  async refreshServerStatus(): Promise<boolean> {
    return invoke('server_status', { identity: this.getIdentity() })
  }

  async refreshServerStatusFor(identity: string): Promise<boolean> {
    return invoke('server_status', { identity })
  }

  async getConfig(): Promise<{ name: string; port: number; font_size: number; notify_target: string; accept_transfers: boolean; transfer_from: string; auto_bump_serial: boolean }> {
    return invoke('get_config', { identity: this.getIdentity() })
  }

  async getConfigFor(identity: string): Promise<{ name: string; port: number; font_size: number; notify_target: string; accept_transfers: boolean; transfer_from: string; auto_bump_serial: boolean }> {
    return invoke('get_config', { identity })
  }

  async saveConfig(config: { port: number; notifyTarget: string; acceptTransfers: boolean; transferFrom: string; autoBumpSerial: boolean }) {
    await invoke('set_config', {
      identity: this.getIdentity(),
      port: config.port,
      notifyTarget: config.notifyTarget,
      acceptTransfers: config.acceptTransfers,
      transferFrom: config.transferFrom,
      autoBumpSerial: config.autoBumpSerial,
    })
  }

  async saveConfigFor(identity: string, config: { port: number; notifyTarget: string; acceptTransfers: boolean; transferFrom: string; autoBumpSerial: boolean }) {
    await invoke('set_config', {
      identity,
      port: config.port,
      notifyTarget: config.notifyTarget,
      acceptTransfers: config.acceptTransfers,
      transferFrom: config.transferFrom,
      autoBumpSerial: config.autoBumpSerial,
    })
  }

  async getAppConfig(): Promise<{ font_size: number }> {
    return invoke('get_app_config')
  }

  async saveAppConfig(config: { fontSize: number }) {
    await invoke('set_app_config', { fontSize: config.fontSize })
  }

  async sendNotify(zoneName: string, target: string): Promise<string> {
    this.addLog({ message: `Sending NOTIFY for ${zoneName} to ${target}`, level: 'info' })
    const result = await invoke<string>('send_notify', { zone: zoneName, target })
    this.addLog({ message: `${result} for ${zoneName}`, level: 'info' })
    return result
  }

  async openWindow(identity: string, port: number) {
    await invoke('open_window', { identity, port })
  }

  async listIdentities(): Promise<Array<{ id: string; name: string }>> {
    return invoke('list_identities')
  }

  async createIdentity(name: string): Promise<{ id: string; name: string }> {
    return invoke('create_identity', { name })
  }

  async renameIdentity(identity: string, newName: string) {
    await invoke('rename_identity', { identity, newName })
  }

  async deleteIdentity(identity: string) {
    await invoke('delete_identity', { identity })
  }

  async restoreZone(zone: { id: string }, oid: string) {
    const identity = this.getIdentity()
    const content = await readBlobAtCommit(identity, oid, zone.id)
    await invoke('save_zone', { identity, name: zone.id, content, port: this.getPort() })
    await invoke('reload_server', { identity })
    this.state.track('zones', zone.id, 'update', { content })
    await commitZone(identity, zone.id, 'Restore', content)
    emit('zones-changed-' + this.getIdentity())
  }

  async pullZone(zone: { id: string }, message?: string): Promise<string> {
    const identity = this.getIdentity()
    const content = await invoke<string>('pull_zone', { identity, name: zone.id, port: this.getPort() })
    try { await commitZone(identity, zone.id, message || `Pull ${zone.id}`, content) } catch (e) { console.error('git commit failed:', e) }
    this.state.track('zones', zone.id, 'update', { content })
    emit('zones-changed-' + identity)
    return content
  }

  async pullZoneFor(identity: string, port: number, zoneName: string, message: string): Promise<string> {
    const content = await invoke<string>('pull_zone', { identity, name: zoneName, port })
    try { await commitZone(identity, zoneName, message, content) } catch (e) { console.error('git commit failed:', e) }
    if (identity === this.getIdentity()) {
      this.state.track('zones', zoneName, 'update', { content })
    }
    return content
  }

  async nukeAppData() {
    await invoke('nuke_app_data')
  }

  addLog(log: { message: string; level: string; server?: string }) {
    if (!log.server) log.server = this.getIdentityName()
    this.state.track('logs', null, 'create', log)
  }
}
