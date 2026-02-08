<script>
export default {
  props: {
    name: { type: String, required: true },
    port: { type: String, required: true },
    acceptTransfers: { type: Boolean, required: true },
    transferFrom: { type: String, required: true },
    otherAddresses: { type: Array, default: () => [] },
    showAddress: { type: Boolean, default: false },
  },

  data() {
    return { copiedAddress: false }
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
  </div>
</template>
