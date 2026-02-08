<script>
import { listen } from '@tauri-apps/api/event'
import EventStore from './state/event-store'
import runners from './state/runners'
import Commands from './commands'
import Queries from './queries'
import ZoneList from './components/ZoneList.vue'
import ZoneEditor from './components/ZoneEditor.vue'
import LogViewer from './components/LogViewer.vue'
import ZoneHistory from './components/ZoneHistory.vue'
import ServerStatus from './components/ServerStatus.vue'
import HelpPane from './components/HelpPane.vue'

export default {
  components: { ZoneList, ZoneEditor, LogViewer, ZoneHistory, ServerStatus, HelpPane },

  data() {
    const params = new URLSearchParams(window.location.search)
    const identity = params.get('identity') || ''
    const state = new EventStore(runners)
    const queries = new Queries({ state })
    const commands = new Commands({ state, queries, getPort: () => this.port, getIdentity: () => this.identity, getIdentityName: () => this.identityName })

    return {
      app: this,
      state,
      queries,
      commands,
      identity,
      identityName: '',
      selectedZone: null,
      serverRunning: false,
      port: 1053,
      bottomTab: 'logs',
      isLoading: true,
      sidebarWidth: 256,
      logHeight: 192,
      logCollapsed: false,
      fontSize: 16,
      notifyTarget: '',
      acceptTransfers: false,
      transferFrom: '',
      autoBumpSerial: true,
      _drag: null,
      showNukeConfirm: false,
    }
  },

  computed: {
    zones() {
      return this.queries.allZones()
    },

    logs() {
      const all = this.queries.allLogs()
      const forIdentity = all.filter(log => !log.identity || log.identity === this.identity)
      return forIdentity
    },

    logCount() {
      if (!this.selectedZone) return this.logs.length
      const name = this.selectedZone.name.toLowerCase()
      return this.logs.filter(log => (log.message || '').toLowerCase().includes(name)).length
    },
  },

  watch: {
    fontSize(val) {
      document.documentElement.style.fontSize = val + 'px'
      this.commands.saveAppConfig({ fontSize: val })
    },
    selectedZone(val) {
      if (!val && this.bottomTab === 'history') this.bottomTab = 'logs'
    },
  },

  async mounted() {
    this.app.selectZone = this.selectZone
    this.app.refreshServerStatus = this.refreshServerStatus
    this.app.toggleLogs = this.toggleLogs
    this.app.switchIdentity = this.switchIdentity
    this.app.clearLogs = this.clearLogs

    if (!this.identity) {
      let identities = await this.commands.listIdentities()
      if (!identities.length) {
        await this.commands.createIdentity('Server')
        identities = await this.commands.listIdentities()
      }
      const first = identities[0]
      this.identity = first?.id || ''
      this.identityName = first?.name || ''
    }

    const params = new URLSearchParams(window.location.search)
    const portOverride = params.get('port')
    if (portOverride) {
      this.port = parseInt(portOverride, 10)
    }

    await Promise.all([
      this.commands.loadZones(),
      this.refreshServerStatus(),
      this.loadConfig(),
    ])
    this.isLoading = false
    this.updateTitle()

    await this._listenToIdentity()

    this._onKeydown = (e) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault()
        this.fontSize = Math.min(28, this.fontSize + 1)
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '-') {
        e.preventDefault()
        this.fontSize = Math.max(10, this.fontSize - 1)
      }
      if ((e.metaKey || e.ctrlKey) && e.key === '0') {
        e.preventDefault()
        this.fontSize = 16
      }
    }
    window.addEventListener('keydown', this._onKeydown)
  },

  unmounted() {
    if (this._unlistenLog) this._unlistenLog()
    if (this._unlistenZones) this._unlistenZones()
    if (this._serverListeners) {
      for (const fn of Object.values(this._serverListeners)) fn()
    }
    if (this._onKeydown) window.removeEventListener('keydown', this._onKeydown)
    this.stopDrag()
  },

  methods: {
    selectZone(zone) {
      const editor = this.$refs.editor
      if (editor?.dirty && !confirm('You have unsaved changes. Discard?')) return
      this.selectedZone = zone
    },

    toggleLogs() {
      this.logCollapsed = !this.logCollapsed
    },

    clearLogs() {
      this.state.clear('logs')
    },

    async refreshServerStatus() {
      this.serverRunning = await this.commands.refreshServerStatus()
    },

    async loadConfig() {
      const [config, appConfig] = await Promise.all([
        this.commands.getConfig(),
        this.commands.getAppConfig(),
      ])
      const params = new URLSearchParams(window.location.search)
      if (!params.get('port')) {
        this.port = config.port
      }
      this.identityName = config.name || this.identity
      this.fontSize = appConfig.font_size
      this.notifyTarget = config.notify_target
      this.acceptTransfers = config.accept_transfers
      this.transferFrom = config.transfer_from
      this.autoBumpSerial = config.auto_bump_serial
    },

    saveConfig() {
      return this.commands.saveConfig({
        port: this.port,
        notifyTarget: this.notifyTarget,
        acceptTransfers: this.acceptTransfers,
        transferFrom: this.transferFrom,
        autoBumpSerial: this.autoBumpSerial,
      })
    },

    async _listenToIdentity() {
      const ident = this.identity
      this._unlistenLog = await listen('log-line-' + ident, (event) => {
        const line = event.payload
        const level = /\[ERROR\]/i.test(line) ? 'error'
          : /\[WARNING\]/i.test(line) ? 'warn'
          : 'info'
        this.commands.addLog({ message: line, level, identity: ident, server: this.identityName })
      })

      this._unlistenZones = await listen('zones-changed-' + this.identity, async () => {
        await this.commands.loadZones()
      })

      await this._listenToServer(ident)
    },

    async _listenToServer(ident) {
      if (!this._serverListeners) this._serverListeners = {}
      if (this._serverListeners[ident]) return

      const config = await this.commands.getConfigFor(ident)
      const port = config.port

      const unSync = await listen('sync-changed-' + ident, async (event) => {
        const zoneName = event.payload
        this.commands.addLog({ message: `Syncing ${zoneName}`, level: 'info', identity: ident })
        try {
          const content = await this.commands.pullZoneFor(ident, port, zoneName, `Sync ${zoneName}`)
          this.commands.addLog({ message: `Synced ${zoneName} (${content.length} bytes)`, level: 'info', identity: ident })
        } catch (e) {
          this.commands.addLog({ message: `Sync failed for ${zoneName}: ${e}`, level: 'error', identity: ident })
        }
      })

      const unExited = await listen('server-exited-' + ident, () => {
        if (ident === this.identity && this.serverRunning) {
          this.serverRunning = false
        }
        this.commands.addLog({ message: 'Server stopped unexpectedly', level: 'error', identity: ident })
      })

      this._serverListeners[ident] = () => { unSync(); unExited() }
    },

    async switchIdentity(newIdentity) {
      if (newIdentity === this.identity) return

      // Unlisten UI-specific listeners (server listeners persist)
      if (this._unlistenLog) this._unlistenLog()
      if (this._unlistenZones) this._unlistenZones()

      // Clear zones (logs persist across identities)
      this.selectedZone = null
      this.state.clear('zones')

      // Switch
      this.identity = newIdentity
      await Promise.all([
        this.commands.loadZones(),
        this.refreshServerStatus(),
        this.loadConfig(),
      ])
      this.updateTitle()

      // Listen to new identity
      await this._listenToIdentity()
    },

    updateTitle() {
      document.title = `ZoneKeeper — ${this.identityName} :${this.port}`
    },

    onVersionClick() {
      this._tapCount = (this._tapCount || 0) + 1
      clearTimeout(this._tapTimer)
      this._tapTimer = setTimeout(() => { this._tapCount = 0 }, 2000)
      if (this._tapCount >= 10) {
        this._tapCount = 0
        this.showNukeConfirm = true
      }
    },

    nukeApp() {
      this.commands.nukeAppData().then(() => window.location.reload())
    },

    startDragSidebar(e) {
      this._drag = { type: 'sidebar', startX: e.clientX, startValue: this.sidebarWidth }
      document.body.style.cursor = 'col-resize'
      document.addEventListener('mousemove', this.onDrag)
      document.addEventListener('mouseup', this.stopDrag)
    },

    startDragLog(e) {
      this._drag = { type: 'log', startY: e.clientY, startValue: this.logHeight }
      document.body.style.cursor = 'row-resize'
      document.addEventListener('mousemove', this.onDrag)
      document.addEventListener('mouseup', this.stopDrag)
    },

    onDrag(e) {
      if (!this._drag) return
      e.preventDefault()

      if (this._drag.type === 'sidebar') {
        const delta = e.clientX - this._drag.startX
        this.sidebarWidth = Math.max(160, Math.min(480, this._drag.startValue + delta))
      }

      if (this._drag.type === 'log') {
        const delta = this._drag.startY - e.clientY
        this.logHeight = Math.max(80, Math.min(600, this._drag.startValue + delta))
      }
    },

    stopDrag() {
      this._drag = null
      document.body.style.cursor = ''
      document.removeEventListener('mousemove', this.onDrag)
      document.removeEventListener('mouseup', this.stopDrag)
    },
  },
}
</script>

<template>
  <div v-if="isLoading" class="flex flex-col items-center justify-center h-screen gap-4">
    <div class="flex gap-1.5">
      <div class="w-2 h-2 rounded-full bg-[#555] animate-pulse" />
      <div class="w-2 h-2 rounded-full bg-[#555] animate-pulse" style="animation-delay: 0.2s" />
      <div class="w-2 h-2 rounded-full bg-[#555] animate-pulse" style="animation-delay: 0.4s" />
    </div>
    <span class="text-xs text-[#555] tracking-wider uppercase">ZoneKeeper</span>
  </div>
  <div
    v-else
    class="flex h-screen"
    :class="{ 'select-none': _drag }"
  >
    <!-- Sidebar -->
    <aside
      class="border-r border-[#3e3e42] flex flex-col shrink-0"
      :style="{ width: sidebarWidth + 'px' }"
    >
      <ServerStatus
        :app="app"
        :serverRunning="serverRunning"
      />
      <ZoneList
        :app="app"
        :zones="zones"
        :selectedZone="selectedZone"
      />
      <div class="px-4 py-2 text-[#666] text-xs select-none cursor-default shrink-0" @click="onVersionClick">v0.1.0</div>
    </aside>

    <!-- Sidebar resize handle -->
    <div
      class="w-1 shrink-0 cursor-col-resize hover:bg-blue-500/40 active:bg-blue-500/60 -ml-1 z-20"
      @mousedown.prevent="startDragSidebar"
    />

    <!-- Main -->
    <main class="flex-1 flex flex-col min-w-0">
      <!-- Editor or Help -->
      <ZoneEditor
        v-if="selectedZone"
        ref="editor"
        :app="app"
        :zone="selectedZone"
        :key="selectedZone.id"
      />
      <HelpPane
        v-else
        :app="app"
        :serverRunning="serverRunning"
        :zoneCount="zones.length"
      />

      <!-- Log resize handle -->
      <div
        v-if="!logCollapsed"
        class="h-1 shrink-0 cursor-row-resize hover:bg-blue-500/40 active:bg-blue-500/60 -mb-1 z-20"
        @mousedown.prevent="startDragLog"
      />

      <!-- Bottom panel -->
      <div
        class="shrink-0 border-t border-[#3e3e42] flex flex-col"
        :style="{ height: logCollapsed ? '32px' : logHeight + 'px' }"
      >
        <!-- Tab bar -->
        <div class="flex items-center justify-between h-9 shrink-0">
          <div class="flex gap-2 pl-2">
            <button
              @click="bottomTab = 'logs'; logCollapsed = false"
              class="px-3 h-9 text-xs font-semibold uppercase tracking-wider cursor-pointer"
              :class="bottomTab === 'logs' && !logCollapsed ? 'text-[#ddd] border-b border-blue-500' : 'text-[#666] hover:text-[#aaa]'"
            >Logs <span v-if="logCount" class="font-normal">({{ logCount }})</span></button>
            <button
              v-if="selectedZone"
              @click="bottomTab = 'history'; logCollapsed = false"
              class="px-3 h-9 text-xs font-semibold uppercase tracking-wider cursor-pointer"
              :class="bottomTab === 'history' && !logCollapsed ? 'text-[#ddd] border-b border-blue-500' : 'text-[#666] hover:text-[#aaa]'"
            >History</button>
          </div>
          <div class="flex items-center">
            <button
              v-if="logs.length && !logCollapsed"
              @click="clearLogs"
              class="px-2 text-xs text-[#555] hover:text-[#aaa] cursor-pointer"
              title="Clear logs"
            >
              <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M8.75 1A2.75 2.75 0 0 0 6 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 1 0 .23 1.482l.149-.022.841 10.518A2.75 2.75 0 0 0 7.596 19h4.807a2.75 2.75 0 0 0 2.742-2.53l.841-10.52.149.023a.75.75 0 0 0 .23-1.482A41.03 41.03 0 0 0 14 4.193V3.75A2.75 2.75 0 0 0 11.25 1h-2.5ZM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4ZM8.58 7.72a.75.75 0 0 0-1.5.06l.3 7.5a.75.75 0 1 0 1.5-.06l-.3-7.5Zm4.34.06a.75.75 0 1 0-1.5-.06l-.3 7.5a.75.75 0 1 0 1.5.06l.3-7.5Z" clip-rule="evenodd"/></svg>
            </button>
            <button
              @click="logCollapsed = !logCollapsed"
              class="px-3 text-xs text-[#666] hover:text-[#aaa] cursor-pointer"
            >{{ logCollapsed ? '&#9650;' : '&#9660;' }}</button>
          </div>
        </div>

        <!-- Tab content -->
        <div v-if="!logCollapsed" class="flex-1 min-h-0">
          <LogViewer
            v-show="bottomTab === 'logs'"
            :app="app"
            :logs="logs"
            :selectedZone="selectedZone"
          />
          <ZoneHistory
            v-if="bottomTab === 'history' && selectedZone"
            :app="app"
            :zone="selectedZone"
            :key="selectedZone.id + '-history'"
          />
        </div>
      </div>
    </main>
  </div>

  <Teleport to="body">
    <div
      v-if="showNukeConfirm"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="showNukeConfirm = false"
      @keydown.escape="showNukeConfirm = false"
    >
      <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-sm">
        <h2 class="text-sm font-semibold text-[#ddd] mb-2">Reset App</h2>
        <p class="text-sm text-[#999] mb-4">Delete all app data? This cannot be undone.</p>
        <div class="flex justify-end gap-2">
          <button
            @click="showNukeConfirm = false"
            class="px-3 py-1.5 text-xs rounded text-[#888] hover:text-white hover:bg-[#2a2d2e] cursor-pointer"
          >Cancel</button>
          <button
            @click="nukeApp()"
            class="px-3 py-1.5 text-xs rounded bg-red-600 hover:bg-red-700 text-white cursor-pointer"
          >Delete Everything</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
