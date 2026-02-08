<script>
import { zoneLog, readBlobAtCommit } from '../git/service'
import { diffLines } from '../git/diff'
import ZoneDiff from './ZoneDiff.vue'

function relativeTime(ms) {
  const seconds = Math.floor((Date.now() - ms) / 1000)
  if (seconds < 60) return 'just now'
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h ago`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days}d ago`
  return new Date(ms).toLocaleDateString()
}

export default {
  components: { ZoneDiff },

  props: {
    app: { type: Object, required: true },
    zone: { type: Object, required: true },
  },

  data() {
    return {
      commits: [],
      loading: true,
      selectedOid: null,
      diffResult: null,
      diffLoading: false,
    }
  },

  watch: {
    'zone.content'() {
      this.loadHistory()
    },
  },

  async mounted() {
    await this.loadHistory()
  },

  methods: {
    async loadHistory() {
      this.loading = true
      try {
        const log = await zoneLog(this.app.identity, this.zone.name)
        this.commits = log.map(entry => ({
          oid: entry.oid,
          message: entry.commit.message.trim(),
          timestamp: entry.commit.author.timestamp * 1000,
        }))
      } catch {
        this.commits = []
      }
      this.loading = false
    },

    async showDiff(commit, index) {
      if (this.selectedOid === commit.oid) {
        this.selectedOid = null
        this.diffResult = null
        return
      }

      this.selectedOid = commit.oid
      this.diffLoading = true

      try {
        const parentCommit = this.commits[index + 1]
        let oldContent = ''
        if (parentCommit) {
          oldContent = await readBlobAtCommit(this.app.identity, parentCommit.oid, this.zone.name)
        }
        const newContent = await readBlobAtCommit(this.app.identity, commit.oid, this.zone.name)
        this.diffResult = diffLines(oldContent, newContent)
      } catch {
        this.diffResult = null
      }
      this.diffLoading = false
    },

    async restore(commit) {
      await this.app.commands.restoreZone(this.zone, commit.oid)
      await this.loadHistory()
    },

    relativeTime,
  },
}
</script>

<template>
  <div class="flex flex-col h-full bg-[#1e1e1e]">
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <span class="text-xs text-[#555]">Loading...</span>
    </div>

    <div v-else-if="commits.length === 0" class="flex-1 flex items-center justify-center">
      <span class="text-xs text-[#555]">No history yet</span>
    </div>

    <div v-else class="flex-1 overflow-y-auto">
      <div
        v-for="(commit, i) in commits"
        :key="commit.oid"
        class="border-b border-[#2d2d30]"
      >
        <div
          class="px-3 py-2 hover:bg-[#2a2d2e] cursor-pointer"
          :class="{ 'bg-[#37373d]': selectedOid === commit.oid }"
          @click="showDiff(commit, i)"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-sm text-[#ccc] truncate">{{ commit.message }}</span>
            <span class="text-xs text-[#666] shrink-0">{{ relativeTime(commit.timestamp) }}</span>
          </div>
          <div class="flex items-center gap-2 mt-1">
            <span class="text-xs text-[#555] font-mono">{{ commit.oid.slice(0, 7) }}</span>
            <button
              v-if="i > 0"
              @click.stop="restore(commit)"
              class="text-xs text-[#888] hover:text-blue-400 cursor-pointer"
            >Restore</button>
          </div>
        </div>

        <div
          v-if="selectedOid === commit.oid"
          class="border-t border-[#2d2d30] bg-[#1a1a1a] max-h-80 overflow-auto"
        >
          <div v-if="diffLoading" class="px-3 py-2 text-xs text-[#555]">Loading diff...</div>
          <ZoneDiff v-else-if="diffResult" :lines="diffResult" />
          <div v-else class="px-3 py-2 text-xs text-[#555]">No changes</div>
        </div>
      </div>
    </div>
  </div>
</template>
