<script>
export default {
  props: {
    app: { type: Object, required: true },
    zones: { type: Array, required: true },
    selectedZone: { type: Object, default: null },
  },

  data() {
    return {
      newZoneName: '',
      isAdding: false,
      confirmingDelete: null,
      search: '',
    }
  },

  computed: {
    filteredZones() {
      const sorted = [...this.zones].sort((a, b) => a.name.localeCompare(b.name))
      if (!this.search) return sorted
      const q = this.search.toLowerCase()
      return sorted.filter(z => z.name.toLowerCase().includes(q))
    },
  },

  methods: {
    startAdding() {
      this.isAdding = true
      this.$nextTick(() => this.$refs.zoneInput?.focus())
    },

    async addZone() {
      const name = this.newZoneName.trim().toLowerCase()
      if (!name) return

      const zone = await this.app.commands.addZone(name)
      this.newZoneName = ''
      this.isAdding = false
      if (zone) this.app.selectZone(zone)
    },

    cancelAdd() {
      this.newZoneName = ''
      this.isAdding = false
    },

    confirmRemove(zone) {
      this.confirmingDelete = zone.id
    },

    async doRemove(zone) {
      this.confirmingDelete = null
      await this.app.commands.removeZone(zone)
      if (this.selectedZone?.id === zone.id) {
        this.app.selectZone(null)
      }
    },
  },
}
</script>

<template>
  <div class="flex-1 overflow-y-auto">
    <div class="flex items-center justify-between px-4 py-2">
      <span class="text-xs font-semibold uppercase tracking-wider text-[#888]">Zones</span>
      <button
        @click="startAdding"
        class="text-[#888] hover:text-white text-lg leading-none"
        aria-label="Add Zone"
      >+</button>
    </div>

    <div v-if="zones.length > 1" class="px-4 pb-2">
      <input
        v-model="search"
        placeholder="Filter..."
        class="w-full bg-[#252526] text-xs px-2 py-1 rounded border border-[#3e3e42] focus:border-blue-400 focus:outline-none text-[#ccc] placeholder-[#555]"
      />
    </div>

    <div
      v-for="zone in filteredZones"
      :key="zone.id"
      class="group flex items-center justify-between px-4 py-2 text-sm cursor-pointer hover:bg-[#2a2d2e]"
      :class="{ 'bg-[#37373d]': selectedZone?.id === zone.id }"
      @click="app.selectZone(selectedZone?.id === zone.id ? null : zone)"
    >
      <span class="flex-1 truncate">{{ zone.name }}</span>
      <span v-if="confirmingDelete === zone.id" class="flex items-center gap-1 ml-2" @click.stop>
        <button
          @click="doRemove(zone)"
          class="text-red-400 hover:text-red-300 text-xs"
        >delete</button>
        <button
          @click="confirmingDelete = null"
          class="text-[#666] hover:text-[#aaa] text-xs"
        >cancel</button>
      </span>
      <button
        v-else
        @click.stop="confirmRemove(zone)"
        class="text-[#666] hover:text-red-400 opacity-0 group-hover:opacity-100 text-xs ml-2"
        aria-label="Remove Zone"
      >&times;</button>
    </div>

    <div v-if="!zones.length && !isAdding" class="px-4 py-8 text-center text-xs text-[#666]">
      No zones configured
    </div>
    <div v-else-if="!filteredZones.length" class="px-4 py-4 text-center text-xs text-[#666]">
      No matches
    </div>
  </div>

  <Teleport to="body">
    <div
      v-if="isAdding"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      @click.self="cancelAdd"
      @keydown.escape="cancelAdd"
    >
      <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-sm">
        <h2 class="text-sm font-semibold text-[#ddd] mb-4">Add Zone</h2>
        <form @submit.prevent="addZone">
          <input
            ref="zoneInput"
            v-model="newZoneName"
            placeholder="example.com"
            class="w-full bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-sm text-[#ccc] focus:border-blue-500 focus:outline-none placeholder-[#555]"
            @keydown.escape="cancelAdd"
          />
          <div class="flex justify-end gap-2 mt-4">
            <button
              type="button"
              @click="cancelAdd"
              class="px-3 py-1.5 text-xs rounded text-[#888] hover:text-white hover:bg-[#2a2d2e] cursor-pointer"
            >Cancel</button>
            <button
              type="submit"
              class="px-3 py-1.5 text-xs rounded bg-blue-600 hover:bg-blue-700 text-white cursor-pointer"
            >Add</button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>
