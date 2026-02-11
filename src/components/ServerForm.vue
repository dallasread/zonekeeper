<script>
export default {
  props: {
    name: { type: String, required: true },
    port: { type: String, required: true },
    acceptTransfers: { type: Boolean, required: true },
    transferFrom: { type: String, required: true },
    notifyServers: { type: Array, default: () => [] },
    otherAddresses: { type: Array, default: () => [] },
    showAddress: { type: Boolean, default: false },
  },

  data() {
    return { copiedAddress: false }
  },

  computed: {
    availableAddresses() {
      const existing = new Set(this.notifyServers.map(r => `${r.ip}:${r.port}`))
      return this.otherAddresses.filter(a => !existing.has(a.address))
    },
  },

  methods: {
    copyAddress() {
      navigator.clipboard.writeText(`127.0.0.1:${this.port}`)
      this.copiedAddress = true
      clearTimeout(this._addrTimer)
      this._addrTimer = setTimeout(() => { this.copiedAddress = false }, 1500)
    },

    emit(field, value) {
      this.$emit('update', { field, value })
    },

    updateRow(index, field, value) {
      const rows = [...this.notifyServers]
      rows[index] = { ...rows[index], [field]: value }
      this.emit('notifyServers', rows)
    },

    addRow() {
      this.emit('notifyServers', [...this.notifyServers, { name: '', ip: '', port: '53' }])
    },

    removeRow(index) {
      this.emit('notifyServers', this.notifyServers.filter((_, i) => i !== index))
    },

    addFromIdentity(a) {
      const c = a.address.lastIndexOf(':')
      const ip = c > 0 ? a.address.slice(0, c) : a.address
      const port = c > 0 ? a.address.slice(c + 1) : '53'
      this.emit('notifyServers', [...this.notifyServers, { name: a.identity, ip, port }])
    },
  },
}
</script>

<template>
  <div>
    <label class="text-xs text-[#888] mb-1 block">Name</label>
    <input
      :value="name"
      @input="emit('name', $event.target.value)"
      class="w-full bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-sm text-[#ccc] focus:border-blue-500 focus:outline-none font-mono mb-3"
      @keydown.enter="$emit('submit')"
    />
    <label class="text-xs text-[#888] mb-1 block">Port</label>
    <input
      :value="port"
      @input="emit('port', $event.target.value)"
      type="number"
      min="1"
      max="65535"
      class="w-full bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-sm text-[#ccc] focus:border-blue-500 focus:outline-none font-mono mb-3"
      @keydown.enter="$emit('submit')"
    />
    <div v-if="showAddress" class="mb-3">
      <label class="text-xs text-[#888] mb-1 block">Address</label>
      <div class="flex items-center bg-[#252526] border border-[#333] rounded overflow-hidden">
        <code class="flex-1 text-xs text-[#ce9178] px-3 py-2 font-mono select-all">127.0.0.1:{{ port }}</code>
        <button
          @click="copyAddress"
          class="px-3 py-2 text-xs border-l border-[#333] shrink-0 cursor-pointer"
          :class="copiedAddress ? 'text-green-400' : 'text-[#888] hover:text-white hover:bg-[#2a2d2e]'"
        >{{ copiedAddress ? 'Copied' : 'Copy' }}</button>
      </div>
    </div>
    <label class="text-xs text-[#888] mb-1 block">Role</label>
    <div class="flex gap-1 mb-3">
      <button
        @click="emit('acceptTransfers', false)"
        class="flex-1 px-2 py-1.5 text-xs rounded cursor-pointer"
        :class="!acceptTransfers ? 'bg-blue-600 text-white' : 'bg-[#252526] text-[#888] hover:text-[#ccc]'"
      >Primary</button>
      <button
        @click="emit('acceptTransfers', true)"
        class="flex-1 px-2 py-1.5 text-xs rounded cursor-pointer"
        :class="acceptTransfers ? 'bg-blue-600 text-white' : 'bg-[#252526] text-[#888] hover:text-[#ccc]'"
      >Secondary</button>
    </div>
    <div v-if="acceptTransfers">
      <label class="text-xs text-[#888] mb-1 block">Primary address</label>
      <input
        :value="transferFrom"
        @input="emit('transferFrom', $event.target.value)"
        list="transfer-from-options"
        placeholder="127.0.0.1:1053"
        class="w-full bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-sm text-[#ccc] focus:border-blue-500 focus:outline-none font-mono mb-3"
        @keydown.enter="$emit('submit')"
      />
      <datalist id="transfer-from-options">
        <option v-for="o in otherAddresses" :key="o.identity" :value="o.address">{{ o.identity }}</option>
      </datalist>
    </div>
    <div v-if="!acceptTransfers">
      <label class="text-xs text-[#888] mb-1 block">Notify servers</label>
      <div class="space-y-1.5 mb-2">
        <div v-for="(row, i) in notifyServers" :key="i" class="flex gap-1.5 items-center">
          <input
            :value="row.name"
            @input="updateRow(i, 'name', $event.target.value)"
            placeholder="Name"
            class="flex-1 min-w-0 bg-[#252526] border border-[#3e3e42] rounded px-2 py-1.5 text-xs text-[#ccc] focus:border-blue-500 focus:outline-none"
          />
          <input
            :value="row.ip"
            @input="updateRow(i, 'ip', $event.target.value)"
            placeholder="IP"
            class="w-28 bg-[#252526] border border-[#3e3e42] rounded px-2 py-1.5 text-xs text-[#ccc] focus:border-blue-500 focus:outline-none font-mono"
          />
          <input
            :value="row.port"
            @input="updateRow(i, 'port', $event.target.value)"
            placeholder="Port"
            type="number"
            min="1"
            max="65535"
            class="w-16 bg-[#252526] border border-[#3e3e42] rounded px-2 py-1.5 text-xs text-[#ccc] focus:border-blue-500 focus:outline-none font-mono"
          />
          <button @click="removeRow(i)" class="text-[#555] hover:text-red-400 cursor-pointer shrink-0" title="Remove">
            <svg class="w-3.5 h-3.5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path d="M6.28 5.22a.75.75 0 0 0-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 1 0 1.06 1.06L10 11.06l3.72 3.72a.75.75 0 1 0 1.06-1.06L11.06 10l3.72-3.72a.75.75 0 0 0-1.06-1.06L10 8.94 6.28 5.22Z"/></svg>
          </button>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-2">
        <button
          @click="addRow"
          class="text-xs text-[#888] hover:text-[#ccc] cursor-pointer"
        >+ Add server</button>
        <button
          v-for="a in availableAddresses"
          :key="a.identity"
          @click="addFromIdentity(a)"
          class="text-xs text-blue-400/70 hover:text-blue-300 cursor-pointer"
        >+ {{ a.identity }}</button>
      </div>
    </div>
  </div>
</template>
