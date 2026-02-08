<script>
import ServerForm from './ServerForm.vue'

export default {
  components: { ServerForm },

  props: {
    app: { type: Object, required: true },
    serverRunning: { type: Boolean, required: true },
  },

  data() {
    return {
      error: null,
      showSettings: false,
      nameDraft: '',
      portDraft: '',
      transferFromDraft: '',
      acceptTransfersDraft: false,
      copiedAddress: false,
      showNewServer: false,
      newName: '',
      newPort: '',
      newAcceptTransfers: false,
      newTransferFrom: '',
      identities: [],
      identityStatuses: {},
      showIdentityMenu: false,
      otherAddresses: [],
      confirmingDelete: false,
    }
  },

  methods: {
    async toggle() {
      this.error = null
      try {
        if (this.serverRunning) {
          await this.app.commands.stopServer()
        } else {
          await this.app.commands.startServer()
        }
      } catch (e) {
        this.error = String(e)
      }
      await this.app.refreshServerStatus()
    },

    async openSettings() {
      this.nameDraft = this.app.identityName
      this.portDraft = String(this.app.port)
      this.acceptTransfersDraft = this.app.acceptTransfers
      this.transferFromDraft = this.app.transferFrom || ''
      this.copiedAddress = false
      this.confirmingDelete = false
      this.showSettings = true

      const ids = await this.app.commands.listIdentities()
      const others = ids.filter(i => i.id !== this.app.identity)
      this.otherAddresses = await Promise.all(others.map(async i => {
        const cfg = await this.app.commands.getConfigFor(i.id)
        return { identity: i.name, address: `127.0.0.1:${cfg.port}` }
      }))
    },

    onSettingsUpdate({ field, value }) {
      if (field === 'name') this.nameDraft = value
      else if (field === 'port') this.portDraft = value
      else if (field === 'acceptTransfers') this.acceptTransfersDraft = value
      else if (field === 'transferFrom') this.transferFromDraft = value
    },

    async saveSettings() {
      const newPort = parseInt(this.portDraft, 10)
      if (!newPort || newPort < 1 || newPort > 65535) return
      const newName = this.nameDraft.trim()
      if (!newName) return

      const nameChanged = newName !== this.app.identityName
      const portChanged = newPort !== this.app.port
      const transferChanged = this.acceptTransfersDraft !== this.app.acceptTransfers
        || this.transferFromDraft.trim() !== (this.app.transferFrom || '')

      this.showSettings = false
      this.error = null

      const needsRestart = (portChanged || transferChanged) && this.serverRunning

      try {
        if (needsRestart) {
          await this.app.commands.stopServer()
          await this.app.refreshServerStatus()
        }

        if (nameChanged) {
          await this.app.commands.renameIdentity(this.app.identity, newName)
          this.app.identityName = newName
        }

        this.app.port = newPort
        this.app.acceptTransfers = this.acceptTransfersDraft
        this.app.transferFrom = this.transferFromDraft.trim()

        await this.app.saveConfig()
        this.app.updateTitle()

        if (needsRestart) {
          await this.app.commands.startServer()
          await this.app.refreshServerStatus()
        }
      } catch (e) {
        this.error = String(e)
        await this.app.refreshServerStatus()
      }
    },

    copyMyAddress() {
      navigator.clipboard.writeText(`127.0.0.1:${this.app.port}`)
      this.copiedAddress = true
      clearTimeout(this._addrTimer)
      this._addrTimer = setTimeout(() => { this.copiedAddress = false }, 1500)
    },

    async toggleIdentityMenu() {
      if (this.showIdentityMenu) {
        this.showIdentityMenu = false
        return
      }
      this.identities = await this.app.commands.listIdentities()
      this.identityStatuses = {}
      this.showIdentityMenu = true
      for (const i of this.identities) {
        this.app.commands.refreshServerStatusFor(i.id).then(running => {
          this.identityStatuses[i.id] = running
        })
      }
    },

    async switchTo(identity) {
      this.showIdentityMenu = false
      await this.app.switchIdentity(identity.id)
    },

    async popOutIdentity(identity) {
      this.showIdentityMenu = false
      try {
        const config = await this.app.commands.getConfigFor(identity.id)
        await this.app.commands.openWindow(identity.id, config.port)
      } catch (e) {
        this.error = String(e)
      }
    },

    async deleteIdentity() {
      if (!this.confirmingDelete) {
        this.confirmingDelete = true
        return
      }
      this.showSettings = false
      this.confirmingDelete = false
      try {
        const current = this.app.identity
        if (this.serverRunning) {
          await this.app.commands.stopServer()
        }
        await this.app.commands.deleteIdentity(current)
        const remaining = await this.app.commands.listIdentities()
        await this.app.switchIdentity(remaining[0].id)
      } catch (e) {
        this.error = String(e)
      }
    },

    async openNewServerDialog() {
      this.showIdentityMenu = false
      this.newName = ''
      this.newPort = String(this.app.port + 1)
      this.newAcceptTransfers = false
      this.newTransferFrom = ''
      this.newAutoBumpSerial = true
      this.showNewServer = true

      const ids = await this.app.commands.listIdentities()
      this.otherAddresses = await Promise.all(ids.map(async i => {
        const cfg = await this.app.commands.getConfigFor(i.id)
        return { identity: i.name, address: `127.0.0.1:${cfg.port}` }
      }))
    },

    onNewServerUpdate({ field, value }) {
      if (field === 'name') this.newName = value
      else if (field === 'port') this.newPort = value
      else if (field === 'acceptTransfers') this.newAcceptTransfers = value
      else if (field === 'transferFrom') this.newTransferFrom = value
    },

    async confirmNewServer() {
      const name = this.newName.trim()
      const port = parseInt(this.newPort, 10)
      if (!name || !port || port < 1 || port > 65535) return
      this.showNewServer = false
      try {
        const identity = await this.app.commands.createIdentity(name)
        await this.app.commands.saveConfigFor(identity.id, {
          port,
          notifyTarget: '',
          acceptTransfers: this.newAcceptTransfers,
          transferFrom: this.newTransferFrom.trim(),
          autoBumpSerial: true,
        })
        await this.app.switchIdentity(identity.id)
      } catch (e) {
        this.error = String(e)
      }
    },
  },
}
</script>

<template>
  <!-- Server header -->
  <div class="px-4 py-2 border-b border-[#3e3e42]">
    <div class="flex items-center justify-between">
      <div class="relative">
        <button
          @click="toggleIdentityMenu"
          class="text-base font-semibold hover:text-[#aaa] cursor-pointer flex items-center gap-1"
        >
          {{ app.identityName }}
          <svg class="w-3 h-3 text-[#666]" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M5.22 8.22a.75.75 0 0 1 1.06 0L10 11.94l3.72-3.72a.75.75 0 1 1 1.06 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L5.22 9.28a.75.75 0 0 1 0-1.06Z" clip-rule="evenodd"/></svg>
        </button>
        <div
          v-if="showIdentityMenu"
          class="absolute top-full left-0 mt-1 bg-[#252526] border border-[#3e3e42] rounded shadow-lg z-50 min-w-[160px]"
        >
          <div
            v-for="i in identities"
            :key="i.id"
            class="flex items-center hover:bg-[#2a2d2e] group"
          >
            <button
              @click="switchTo(i)"
              class="flex-1 text-left px-3 py-1.5 text-xs cursor-pointer flex items-center gap-2"
              :class="i.id === app.identity ? 'text-blue-400' : 'text-[#ccc]'"
            >
              <span
                class="w-1.5 h-1.5 rounded-full shrink-0"
                :class="identityStatuses[i.id] ? 'bg-green-400' : 'bg-[#555]'"
              ></span>
              {{ i.name }}
            </button>
            <button
              @click.stop="popOutIdentity(i)"
              class="px-2 py-1.5 text-[#555] hover:text-[#ccc] cursor-pointer opacity-0 group-hover:opacity-100"
              title="Open in new window"
            >
              <svg class="w-3 h-3" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .414.336.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h5a.75.75 0 0 1 0 1.5h-5Zm10.22-2.03a.75.75 0 0 1 .53-.22h3.5a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0V5.56l-5.72 5.72a.75.75 0 1 1-1.06-1.06l5.72-5.72h-2.22a.75.75 0 0 1-.75-.75Z" clip-rule="evenodd"/></svg>
            </button>
          </div>
          <div class="border-t border-[#3e3e42]">
            <button
              @click="openNewServerDialog"
              class="w-full text-left px-3 py-1.5 text-xs text-[#888] hover:text-[#ccc] hover:bg-[#2a2d2e] cursor-pointer"
            >New server...</button>
          </div>
        </div>
      </div>
      <button
        @click="toggle"
        class="px-2 py-0.5 text-[11px] rounded cursor-pointer"
        :class="serverRunning
          ? 'bg-red-900/50 text-red-300 hover:bg-red-900/70'
          : 'bg-green-900/50 text-green-300 hover:bg-green-900/70'"
      >
        {{ serverRunning ? 'Stop' : 'Start' }}
      </button>
    </div>
    <div class="flex items-center justify-between mt-1">
      <button
        @click="copyMyAddress"
        class="text-xs font-mono text-[#888] hover:text-[#ccc] cursor-pointer"
        :title="copiedAddress ? 'Copied!' : 'Click to copy'"
      >{{ copiedAddress ? 'Copied!' : '127.0.0.1:' + app.port }}</button>
      <button
        @click="openSettings"
        class="text-[#555] hover:text-[#aaa] cursor-pointer"
        title="Settings"
      >
        <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M7.84 1.804A1 1 0 0 1 8.82 1h2.36a1 1 0 0 1 .98.804l.331 1.652a6.993 6.993 0 0 1 1.929 1.115l1.598-.54a1 1 0 0 1 1.186.447l1.18 2.044a1 1 0 0 1-.205 1.251l-1.267 1.113a7.047 7.047 0 0 1 0 2.228l1.267 1.113a1 1 0 0 1 .206 1.25l-1.18 2.045a1 1 0 0 1-1.187.447l-1.598-.54a6.993 6.993 0 0 1-1.929 1.115l-.33 1.652a1 1 0 0 1-.98.804H8.82a1 1 0 0 1-.98-.804l-.331-1.652a6.993 6.993 0 0 1-1.929-1.115l-1.598.54a1 1 0 0 1-1.186-.447l-1.18-2.044a1 1 0 0 1 .205-1.251l1.267-1.114a7.05 7.05 0 0 1 0-2.227L1.821 7.773a1 1 0 0 1-.206-1.25l1.18-2.045a1 1 0 0 1 1.187-.447l1.598.54A6.992 6.992 0 0 1 7.51 3.456l.33-1.652ZM10 13a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z" clip-rule="evenodd"/></svg>
      </button>
    </div>
    <div v-if="error" class="text-xs text-red-400 mt-1">{{ error }}</div>
  </div>

  <!-- Click outside to close menu -->
  <Teleport to="body">
    <div
      v-if="showIdentityMenu"
      class="fixed inset-0 z-40"
      @click="showIdentityMenu = false"
    />
  </Teleport>

  <!-- Settings dialog -->
  <Teleport to="body">
    <div
      v-if="showSettings"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="showSettings = false"
      @keydown.escape="showSettings = false"
    >
      <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-xs">
        <h2 class="text-sm font-semibold text-[#ddd] mb-4">Server Settings</h2>
        <ServerForm
          :name="nameDraft"
          :port="portDraft"
          :acceptTransfers="acceptTransfersDraft"
          :transferFrom="transferFromDraft"
          :otherAddresses="otherAddresses"
          :showAddress="true"
          @update="onSettingsUpdate"
          @submit="saveSettings"
        />
        <div class="border-t border-[#3e3e42] pt-3 mb-3">
          <button
            @click="deleteIdentity"
            class="text-xs cursor-pointer"
            :class="confirmingDelete ? 'text-red-400 hover:text-red-300' : 'text-[#555] hover:text-red-400'"
          >{{ confirmingDelete ? 'Click again to confirm deletion' : 'Delete this server...' }}</button>
        </div>
        <div class="flex justify-end gap-2">
          <button
            @click="showSettings = false"
            class="px-3 py-1.5 text-xs rounded text-[#888] hover:text-white hover:bg-[#2a2d2e] cursor-pointer"
          >Cancel</button>
          <button
            @click="saveSettings"
            class="px-3 py-1.5 text-xs rounded bg-blue-600 hover:bg-blue-700 text-white cursor-pointer"
          >Save</button>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- New server dialog -->
  <Teleport to="body">
    <div
      v-if="showNewServer"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="showNewServer = false"
      @keydown.escape="showNewServer = false"
    >
      <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-xs">
        <h2 class="text-sm font-semibold text-[#ddd] mb-4">New Server</h2>
        <ServerForm
          :name="newName"
          :port="newPort"
          :acceptTransfers="newAcceptTransfers"
          :transferFrom="newTransferFrom"
          :otherAddresses="otherAddresses"
          @update="onNewServerUpdate"
          @submit="confirmNewServer"
        />
        <div class="flex justify-end gap-2">
          <button
            @click="showNewServer = false"
            class="px-3 py-1.5 text-xs rounded text-[#888] hover:text-white hover:bg-[#2a2d2e] cursor-pointer"
          >Cancel</button>
          <button
            @click="confirmNewServer"
            class="px-3 py-1.5 text-xs rounded bg-blue-600 hover:bg-blue-700 text-white cursor-pointer"
            :disabled="!newName.trim()"
          >Create</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
