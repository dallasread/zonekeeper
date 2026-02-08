<script>
function parseLog(log) {
  let body = log.message
  let level = log.level

  // Messages with [ERROR]/[WARNING] prefix from Rust
  if (body.startsWith('[ERROR] ')) {
    level = 'error'
    body = body.slice(8)
  } else if (body.startsWith('[WARNING] ')) {
    level = 'warn'
    body = body.slice(10)
  }

  // Detect NOTIFY log entries with a zone name
  let notifyZone = null
  const notifyMatch = body.match(/NOTIFY\S*\s+.*?for\s+(\S+)/) || body.match(/^Received NOTIFY for (\S+)/)
  if (notifyMatch) {
    notifyZone = notifyMatch[1].toLowerCase()
  }

  return { level, server: log.server || '', body, notifyZone, createdAt: log.createdAt }
}

export default {
  props: {
    app: { type: Object, required: true },
    logs: { type: Array, required: true },
    selectedZone: { type: Object, default: null },
  },

  data() {
    return {
      levelFilter: 'all',
    }
  },

  computed: {
    parsed() {
      return this.logs.map(parseLog).filter(Boolean)
    },

    filtered() {
      let result = this.parsed
      if (this.levelFilter !== 'all') {
        result = result.filter(l => l.level === this.levelFilter)
      }
      if (this.selectedZone) {
        const name = this.selectedZone.name.toLowerCase()
        result = result.filter(l => l.body.toLowerCase().includes(name))
      }
      return result
    },

    counts() {
      const c = { error: 0, warn: 0, info: 0 }
      for (const l of this.parsed) c[l.level] = (c[l.level] || 0) + 1
      return c
    },

    multipleServers() {
      const names = new Set(this.parsed.map(l => l.server).filter(Boolean))
      return names.size > 1
    },
  },

  watch: {
    filtered() {
      this.$nextTick(() => {
        const el = this.$refs.logContainer
        if (el) el.scrollTop = el.scrollHeight
      })
    },
  },

  methods: {
    toggleFilter(level) {
      this.levelFilter = this.levelFilter === level ? 'all' : level
    },

    formatTime(ts) {
      if (!ts) return ''
      const d = new Date(ts)
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })
    },

    async createAndPull(zoneName) {
      try {
        const zone = await this.app.commands.addZone(zoneName)
        if (zone) {
          this.app.selectZone(zone)
          await this.app.commands.pullZone(zone)
        }
      } catch (e) {
        this.app.commands.addLog({ message: `Failed to create ${zoneName}: ${e}`, level: 'error' })
      }
    },

    zoneExists(name) {
      const lower = name.toLowerCase()
      return this.app.queries.allZones().some(z => z.name.toLowerCase() === lower)
    },
  },
}
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- Filter bar -->
    <div v-if="parsed.length" class="flex items-center gap-1 px-4 py-1 shrink-0">
      <button
        v-for="({ key, label, color }) in [
          { key: 'error', label: 'Error', color: 'text-red-400' },
          { key: 'warn', label: 'Warn', color: 'text-yellow-400' },
        ]"
        :key="key"
        v-show="counts[key]"
        @click="toggleFilter(key)"
        class="px-1.5 py-0.5 text-[0.65em] rounded cursor-pointer border"
        :class="levelFilter === key
          ? `${color} border-current`
          : 'text-[#555] border-[#333] hover:text-[#888]'"
      >{{ label }} ({{ counts[key] }})</button>
    </div>

    <!-- Log entries -->
    <div
      ref="logContainer"
      class="flex-1 overflow-y-auto font-mono text-sm leading-5 px-4"
    >
      <div v-if="!logs.length" class="text-[#666] py-2">
        No log entries yet. Start the server to see queries.
      </div>
      <div
        v-for="(log, i) in filtered"
        :key="i"
        class="flex items-baseline gap-2 py-1"
        :class="i % 2 === 1 ? 'bg-[#1a1a1a]' : ''"
      >
        <span class="text-[#444] shrink-0 w-16 text-right select-none">{{ formatTime(log.createdAt) }}</span>
        <span
          v-if="log.level === 'error'"
          class="text-[0.65em] text-red-400 shrink-0 w-8"
        >ERR</span>
        <span
          v-else-if="log.level === 'warn'"
          class="text-[0.65em] text-yellow-400 shrink-0 w-8"
        >WRN</span>
        <span v-else class="shrink-0 w-8"></span>
        <span v-if="multipleServers" class="text-[#555] shrink-0 w-24 truncate" :title="log.server">{{ log.server }}</span>
        <span
          class="flex-1 min-w-0"
          :class="{
            'text-red-300': log.level === 'error',
            'text-yellow-300': log.level === 'warn',
            'text-[#999]': log.level === 'info',
          }"
        >{{ log.body }}
          <button
            v-if="log.notifyZone && !zoneExists(log.notifyZone)"
            @click="createAndPull(log.notifyZone)"
            class="text-xs text-blue-300 bg-blue-900/40 hover:bg-blue-900/60 rounded px-2 py-0.5 cursor-pointer ml-2"
          >+ Add to {{ app.identityName }}</button>
        </span>
      </div>
    </div>
  </div>
</template>
