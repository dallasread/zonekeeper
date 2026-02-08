<script>
import { VALID_TYPES, VALID_TYPES_ARRAY, CLASSES, validateContent } from '../editor/validation'
import { highlightLine } from '../editor/highlight'
import { getContext, getSuggestions } from '../editor/autocomplete'

const RECORD_TEMPLATES = [
  { label: 'A',     text: 'name  3600  IN  A  127.0.0.1' },
  { label: 'AAAA',  text: 'name  3600  IN  AAAA  ::1' },
  { label: 'CNAME', text: 'name  3600  IN  CNAME  target.example.com.' },
  { label: 'MX',    text: '@  3600  IN  MX  10  mail.example.com.' },
  { label: 'TXT',   text: '@  3600  IN  TXT  "v=spf1 include:_spf.example.com ~all"' },
  { label: 'NS',    text: '@  3600  IN  NS  ns1.example.com.' },
  { label: 'SRV',   text: '_svc._tcp  3600  IN  SRV  10  0  443  target.example.com.' },
  { label: 'CAA',   text: '@  3600  IN  CAA  0  issue  "letsencrypt.org"' },
  { label: 'PTR',   text: '1  3600  IN  PTR  host.example.com.' },
  { label: 'DNAME', text: 'name  3600  IN  DNAME  target.example.com.' },
  { label: 'HTTPS', text: '@  3600  IN  HTTPS  1  .  alpn="h2,h3"' },
  { label: 'SSHFP', text: '@  3600  IN  SSHFP  1  1  abc123' },
  { label: 'TLSA',  text: '_443._tcp  3600  IN  TLSA  3  1  1  abc123' },
  { label: 'DS',    text: '@  3600  IN  DS  12345  13  2  abc123' },
  { label: 'NAPTR', text: '@  3600  IN  NAPTR  10  10  "u"  "E2U+sip"  "!^.*$!sip:info@example.com!"  .' },
  { label: 'LOC',   text: '@  3600  IN  LOC  52 22 23.000 N 4 53 32.000 E 0.00m' },
]

export default {

  props: {
    app: { type: Object, required: true },
    zone: { type: Object, required: true },
  },

  data() {
    return {
      content: this.zone.content || '',
      errors: [],
      saving: false,
      saved: false,
      saveMessage: '',
      commitMsg: '',
      showDigInfo: false,
      copiedCmd: null,
      showSaveDialog: false,
      bumpSerial: true,
      showNotifyDialog: false,
      notifyTarget: '',
      notifySending: false,
      notifyStatus: null,
      notifyAddresses: [],
      pulling: false,
      showTemplates: false,
      acSuggestions: [],
      acIndex: 0,
      acVisible: false,
      acStyle: null,
      acContext: null,
    }
  },

  computed: {
    isSecondary() {
      return this.app.acceptTransfers
    },

    dirty() {
      return this.content !== this.zone.content
    },

    lines() {
      return this.content.split('\n')
    },

    highlightedLines() {
      return this.lines.map(l => highlightLine(l))
    },

    errorsByLine() {
      const map = {}
      for (const err of this.errors) {
        map[err.line] = err
      }
      return map
    },

    templates() {
      return RECORD_TEMPLATES
    },

    zoneHostnames() {
      const names = new Set()
      for (const line of this.lines) {
        if (!line || line.startsWith(' ') || line.startsWith('\t')) continue
        const trimmed = line.trim()
        if (!trimmed || trimmed.startsWith(';') || trimmed.startsWith('$')) continue
        const first = trimmed.split(/\s+/)[0]
        if (first) names.add(first)
      }
      return [...names]
    },

    digCommands() {
      const zone = this.zone.name
      const base = `dig @127.0.0.1 -p ${this.app.port}`
      const records = []
      const seen = new Set()
      let lastHost = '@'

      for (const line of this.lines) {
        const trimmed = line.trim()
        if (!trimmed || trimmed.startsWith(';') || trimmed.startsWith('$')) continue
        const withoutComment = trimmed.replace(/\s*;.*$/, '')
        const tokens = withoutComment.split(/\s+/).filter(Boolean)
        if (tokens.length < 2) continue

        let i = 0
        let host = lastHost

        // First token is hostname if line doesn't start with whitespace
        if (!line.startsWith(' ') && !line.startsWith('\t')) {
          host = tokens[0]
          lastHost = host
          i = 1
        }

        // Skip TTL, class to find type
        while (i < tokens.length) {
          const upper = tokens[i].toUpperCase()
          if (/^\d+[smhdw]?$/i.test(tokens[i]) || CLASSES.has(upper)) { i++; continue }
          if (VALID_TYPES.has(upper) && upper !== 'SOA' && upper !== 'NS') {
            const fqdn = host === '@' ? zone : host.endsWith('.') ? host.slice(0, -1) : `${host}.${zone}`
            const key = `${fqdn} ${upper}`
            if (!seen.has(key)) {
              seen.add(key)
              records.push({ host: fqdn, type: upper })
            }
          }
          break
        }
      }

      const commands = [
        { label: 'SOA', cmd: `${base} ${zone} SOA` },
        { label: 'NS', cmd: `${base} ${zone} NS` },
        { label: 'AXFR', cmd: `${base} axfr ${zone}` },
      ]

      for (const r of records) {
        commands.push({ label: `${r.host} ${r.type}`, cmd: `${base} ${r.host} ${r.type}` })
      }

      return commands
    },
  },

  watch: {
    content() {
      this.errors = validateContent(this.content)
    },
    'zone.content'(val) {
      if (val !== this.content) {
        this.content = val
      }
    },
    showSaveDialog(val) {
      if (val) {
        this.bumpSerial = true
        this.$nextTick(() => this.$refs.commitInput?.focus())
      }
    },
    async showNotifyDialog(val) {
      if (val) {
        this.$nextTick(() => this.$refs.notifyInput?.focus())
        const ids = await this.app.commands.listIdentities()
        const others = ids.filter(i => i.id !== this.app.identity)
        this.notifyAddresses = await Promise.all(others.map(async i => {
          const cfg = await this.app.commands.getConfigFor(i.id)
          return { identity: i.name, address: `127.0.0.1:${cfg.port}` }
        }))
      }
    },
  },

  async mounted() {
    this.errors = validateContent(this.content)
    this.notifyTarget = this.app.notifyTarget || ''
    try {
      const fresh = await this.app.commands.readZone(this.zone.id)
      if (fresh && fresh !== this.content) {
        this.content = fresh
      }
    } catch (e) { /* zone may not exist on disk yet */ }
    this._beforeUnload = (e) => {
      if (this.dirty) {
        e.preventDefault()
        e.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', this._beforeUnload)
  },

  unmounted() {
    window.removeEventListener('beforeunload', this._beforeUnload)
  },

  methods: {
    _replaceLineViaTextarea(lineIndex, newLine) {
      const textarea = this.$el.querySelector('textarea')
      const lines = this.content.split('\n')
      let start = 0
      for (let i = 0; i < lineIndex; i++) {
        start += lines[i].length + 1
      }
      const end = start + lines[lineIndex].length
      textarea.focus()
      textarea.setSelectionRange(start, end)
      document.execCommand('insertText', false, newLine)
    },

    applyFix(error) {
      const lines = this.content.split('\n')
      const idx = error.line - 1
      if (idx >= 0 && idx < lines.length && error.fix) {
        this._replaceLineViaTextarea(idx, error.fix(lines[idx]))
      }
    },

    applyAllFixes() {
      const sorted = [...this.errors].filter(e => e.fix).sort((a, b) => b.line - a.line)
      for (const error of sorted) {
        const lines = this.content.split('\n')
        const idx = error.line - 1
        if (idx >= 0 && idx < lines.length) {
          this._replaceLineViaTextarea(idx, error.fix(lines[idx]))
        }
      }
    },

    async save() {
      if (this.errors.length) return
      this.saving = true
      this.app.autoBumpSerial = this.bumpSerial
      this.app.saveConfig()
      const msg = this.commitMsg.trim() || `Update ${this.zone.name}`
      await this.app.commands.updateZone(this.zone, this.content, msg)
      this.commitMsg = ''
      this.showSaveDialog = false
      this.saving = false
      this.saved = true
      clearTimeout(this._savedTimer)
      this._savedTimer = setTimeout(() => { this.saved = false }, 2000)
    },

    flashMessage(msg) {
      this.saveMessage = msg
      clearTimeout(this._msgTimer)
      this._msgTimer = setTimeout(() => { this.saveMessage = '' }, 2500)
    },

    trySave() {
      if (!this.dirty) return
      if (this.errors.length) {
        this.flashMessage(`${this.errors.length} validation error${this.errors.length === 1 ? '' : 's'}`)
        return
      }
      this.showSaveDialog = true
    },

    insertTemplate(tpl) {
      this.showTemplates = false
      const textarea = this.$refs.textarea
      textarea.focus()
      const pos = textarea.selectionStart
      // If not at start of line, prepend newline
      const before = this.content.slice(0, pos)
      const needsNewline = before.length > 0 && !before.endsWith('\n')
      const text = (needsNewline ? '\n' : '') + tpl.text
      document.execCommand('insertText', false, text)
      // Select the inserted text for easy editing
      const insertStart = pos + (needsNewline ? 1 : 0)
      this.$nextTick(() => {
        textarea.setSelectionRange(insertStart, insertStart + tpl.text.length)
      })
    },

    onInput() {
      this.updateAutocomplete()
    },

    updateAutocomplete() {
      const textarea = this.$refs.textarea
      if (!textarea) return
      const ctx = getContext(this.content, textarea.selectionStart)
      if (!ctx) {
        this.acVisible = false
        return
      }
      const suggestions = getSuggestions(ctx, this.zoneHostnames)
      if (!suggestions.length) {
        this.acVisible = false
        return
      }
      this.acContext = ctx
      this.acSuggestions = suggestions
      this.acIndex = 0
      this.acStyle = this._getCursorPosition()
      this.acVisible = true
    },

    _getCursorPosition() {
      const textarea = this.$refs.textarea
      const mirror = this.$refs.mirror
      if (!textarea || !mirror) return null
      const text = this.content.slice(0, textarea.selectionStart)
      mirror.textContent = text
      const marker = document.createElement('span')
      marker.textContent = '|'
      mirror.appendChild(marker)
      const textareaRect = textarea.getBoundingClientRect()
      const markerRect = marker.getBoundingClientRect()
      return {
        position: 'fixed',
        left: Math.min(markerRect.left, textareaRect.right - 200) + 'px',
        top: (markerRect.bottom + 2) + 'px',
        zIndex: 60,
      }
    },

    acceptSuggestion(value) {
      const textarea = this.$refs.textarea
      if (!textarea || !this.acContext) return
      const { startPos, prefix } = this.acContext
      textarea.focus()
      textarea.setSelectionRange(startPos, startPos + prefix.length)
      document.execCommand('insertText', false, value)
      this.acVisible = false
    },

    onKeydown(e) {
      // Autocomplete keyboard handling
      if (this.acVisible) {
        if (e.key === 'ArrowDown') {
          e.preventDefault()
          this.acIndex = (this.acIndex + 1) % this.acSuggestions.length
          return
        }
        if (e.key === 'ArrowUp') {
          e.preventDefault()
          this.acIndex = (this.acIndex - 1 + this.acSuggestions.length) % this.acSuggestions.length
          return
        }
        if (e.key === 'Enter' || e.key === 'Tab') {
          e.preventDefault()
          this.acceptSuggestion(this.acSuggestions[this.acIndex])
          return
        }
        if (e.key === 'Escape') {
          e.preventDefault()
          this.acVisible = false
          return
        }
      }

      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault()
        this.trySave()
      }

      if (e.key === 'Tab') {
        e.preventDefault()
        document.execCommand('insertText', false, '  ')
      }
    },

    copyCmd(cmd) {
      navigator.clipboard.writeText(cmd)
      this.copiedCmd = cmd
      clearTimeout(this._copyTimer)
      this._copyTimer = setTimeout(() => { this.copiedCmd = null }, 1500)
    },

    async pullFromServer() {
      this.pulling = true
      try {
        const content = await this.app.commands.pullZone(this.zone)
        this.content = content
        this.flashMessage('Pulled from server')
      } catch (e) {
        this.flashMessage(String(e))
      }
      this.pulling = false
    },

    async sendNotify() {
      const target = this.notifyTarget.trim()
      if (!target) return
      this.notifySending = true
      this.notifyStatus = null
      try {
        this.app.notifyTarget = target
        this.app.saveConfig()
        const result = await this.app.commands.sendNotify(this.zone.name, target)
        this.notifyStatus = { ok: true, message: result }
        setTimeout(() => { this.showNotifyDialog = false }, 800)
      } catch (e) {
        this.notifyStatus = { ok: false, message: String(e) }
      }
      this.notifySending = false
    },

    onScroll(e) {
      this.$refs.backdrop.scrollTop = e.target.scrollTop
      this.$refs.backdrop.scrollLeft = e.target.scrollLeft
      this.$refs.lineNumbers.scrollTop = e.target.scrollTop
    },

  },
}
</script>

<template>
  <div class="flex-1 flex flex-col min-h-0">
    <!-- Header -->
    <div class="flex items-center justify-between px-4 py-2 border-b border-[#3e3e42]">
      <div class="flex items-center gap-2">
        <span class="text-sm font-medium">{{ zone.name }}</span>
        <button
          @click="showDigInfo = !showDigInfo"
          class="text-[#666] hover:text-[#ccc] cursor-pointer"
          title="dig commands"
        ><svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path fill-rule="evenodd" d="M3.25 3A2.25 2.25 0 0 0 1 5.25v9.5A2.25 2.25 0 0 0 3.25 17h13.5A2.25 2.25 0 0 0 19 14.75v-9.5A2.25 2.25 0 0 0 16.75 3H3.25Zm.943 8.752a.75.75 0 0 1 .055-1.06L6.128 9l-1.88-1.693a.75.75 0 1 1 1.004-1.114l2.5 2.25a.75.75 0 0 1 0 1.114l-2.5 2.25a.75.75 0 0 1-1.06-.055ZM9.75 10.25a.75.75 0 0 0 0 1.5h2.5a.75.75 0 0 0 0-1.5h-2.5Z" clip-rule="evenodd"/></svg></button>
        <button
          v-if="isSecondary"
          @click="pullFromServer"
          :disabled="pulling"
          class="text-[#666] hover:text-[#ccc] cursor-pointer disabled:opacity-50"
          title="Pull from server (AXFR)"
        ><svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path d="M10.75 2.75a.75.75 0 0 0-1.5 0v8.614L6.295 8.235a.75.75 0 1 0-1.09 1.03l4.25 4.5a.75.75 0 0 0 1.09 0l4.25-4.5a.75.75 0 0 0-1.09-1.03l-2.955 3.129V2.75Z"/><path d="M3.5 12.75a.75.75 0 0 0-1.5 0v2.5A2.75 2.75 0 0 0 4.75 18h10.5A2.75 2.75 0 0 0 18 15.25v-2.5a.75.75 0 0 0-1.5 0v2.5c0 .69-.56 1.25-1.25 1.25H4.75c-.69 0-1.25-.56-1.25-1.25v-2.5Z"/></svg></button>
        <!-- Insert record template -->
        <div v-if="!isSecondary" class="relative flex items-center">
          <button
            @click="showTemplates = !showTemplates"
            class="text-[#666] hover:text-[#ccc] cursor-pointer"
            title="Insert record template"
          ><svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor"><path d="M10.75 4.75a.75.75 0 0 0-1.5 0v4.5h-4.5a.75.75 0 0 0 0 1.5h4.5v4.5a.75.75 0 0 0 1.5 0v-4.5h4.5a.75.75 0 0 0 0-1.5h-4.5v-4.5Z"/></svg></button>
          <div
            v-if="showTemplates"
            class="absolute top-full left-0 mt-1 bg-[#252526] border border-[#3e3e42] rounded shadow-lg z-50 min-w-[280px]"
          >
            <button
              v-for="tpl in templates"
              :key="tpl.label"
              @click="insertTemplate(tpl)"
              class="w-full text-left px-3 py-1.5 text-xs hover:bg-[#2a2d2e] cursor-pointer flex items-center gap-2"
            >
              <span class="text-blue-400 font-mono w-12 shrink-0">{{ tpl.label }}</span>
              <span class="text-[#888] font-mono truncate">{{ tpl.text }}</span>
            </button>
          </div>
        </div>
      </div>
      <div class="flex items-center gap-3">
        <span v-if="saveMessage" class="text-xs text-yellow-400">{{ saveMessage }}</span>
        <template v-if="!isSecondary">
          <button
            v-if="errors.length > 1"
            @click="applyAllFixes"
            class="px-2 py-1 text-xs rounded bg-red-900/40 text-red-300 hover:bg-red-900/60"
          >
            Fix all ({{ errors.length }})
          </button>
          <span v-if="errors.length" class="text-xs text-red-400">
            {{ errors.length }} error{{ errors.length === 1 ? '' : 's' }}
          </span>
          <span v-if="saved" class="text-xs text-green-400">Saved</span>
          <button
            @click="showNotifyDialog = true"
            :disabled="dirty"
            class="px-3 py-1 text-xs rounded text-white disabled:opacity-50 disabled:cursor-not-allowed bg-amber-600 hover:bg-amber-700 cursor-pointer"
          >Notify</button>
          <button
            @click="trySave"
            :disabled="saving || !dirty"
            class="px-3 py-1 text-xs rounded text-white disabled:opacity-50 disabled:cursor-not-allowed bg-blue-600 hover:bg-blue-700 cursor-pointer"
          >
            {{ saving ? 'Saving...' : 'Save' }}
          </button>
        </template>
      </div>
    </div>

    <!-- Editor + History -->
    <div class="flex-1 flex min-h-0">
    <!-- Editor -->
    <div class="flex-1 flex overflow-hidden relative font-mono text-sm leading-5">
      <!-- Line numbers -->
      <div
        ref="lineNumbers"
        class="py-2 text-right text-[#666] select-none border-r border-[#3e3e42] bg-[#1e1e1e] overflow-hidden shrink-0"
      >
        <div
          v-for="(_, i) in lines"
          :key="i"
          class="px-3"
          :class="errorsByLine[i + 1] ? 'text-red-400 bg-red-900/20' : i % 2 === 1 ? 'bg-[#1a1a1a]' : ''"
        >{{ i + 1 }}</div>
      </div>

      <!-- Editor area -->
      <div class="flex-1 relative overflow-hidden">
        <!-- Styled backdrop -->
        <div
          ref="backdrop"
          class="absolute inset-0 overflow-hidden py-2"
        >
          <div
            v-for="(line, i) in lines"
            :key="i"
            class="px-4 whitespace-pre relative"
            :class="errorsByLine[i + 1] ? 'bg-red-900/20' : i % 2 === 1 ? 'bg-[#1a1a1a]' : ''"
          >
            <span
              class="pointer-events-none"
              :class="{ 'text-red-300': errorsByLine[i + 1] }"
              v-html="errorsByLine[i + 1] ? (line || ' ') : highlightedLines[i]"
            ></span>
            <span
              v-if="errorsByLine[i + 1]"
              class="pointer-events-auto ml-4 text-xs text-red-400/80 whitespace-nowrap"
            >{{ errorsByLine[i + 1].message }}<button
                v-if="errorsByLine[i + 1].fixLabel"
                @click="applyFix(errorsByLine[i + 1])"
                class="ml-1.5 text-red-300 bg-red-900/50 hover:bg-red-800/60 px-1 rounded cursor-pointer"
              >{{ errorsByLine[i + 1].fixLabel }}</button></span>
          </div>
        </div>

        <!-- Hidden mirror for cursor positioning -->
        <div
          ref="mirror"
          class="absolute top-0 left-0 px-4 py-2 font-mono text-sm leading-5 whitespace-pre invisible overflow-hidden pointer-events-none"
          aria-hidden="true"
        ></div>

        <!-- Transparent textarea -->
        <textarea
          ref="textarea"
          v-model="content"
          :readonly="isSecondary"
          @keydown="onKeydown"
          @input="onInput"
          @scroll="onScroll"
          @click="acVisible = false"
          spellcheck="false"
          class="absolute inset-0 w-full h-full resize-none bg-transparent text-transparent caret-white px-4 py-2 leading-5 focus:outline-none font-mono text-sm whitespace-pre overflow-auto"
        />

      </div>
    </div>

    </div>

    <!-- Click outside to close templates -->
    <Teleport to="body">
      <div v-if="showTemplates" class="fixed inset-0 z-40" @click="showTemplates = false" />
    </Teleport>

    <!-- Autocomplete dropdown -->
    <Teleport to="body">
      <div
        v-if="acVisible && acSuggestions.length"
        class="bg-[#252526] border border-[#3e3e42] rounded shadow-lg max-h-[200px] overflow-y-auto min-w-[140px]"
        :style="acStyle"
      >
        <div
          v-for="(s, i) in acSuggestions"
          :key="s"
          class="px-3 py-1 text-xs font-mono cursor-pointer"
          :class="i === acIndex ? 'bg-blue-600 text-white' : 'text-[#ccc] hover:bg-[#2a2d2e]'"
          @mousedown.prevent="acceptSuggestion(s)"
        >{{ s }}</div>
      </div>
    </Teleport>

    <!-- Dig commands dialog -->
    <Teleport to="body">
      <div
        v-if="showDigInfo"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click.self="showDigInfo = false"
      >
        <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-xl max-h-[80vh] overflow-y-auto">
          <div class="flex items-center justify-between mb-4">
            <h2 class="text-sm font-semibold text-[#ddd]">dig commands for {{ zone.name }}</h2>
            <button @click="showDigInfo = false" class="text-[#666] hover:text-white text-lg cursor-pointer leading-none">&times;</button>
          </div>
          <div class="space-y-2">
            <div
              v-for="d in digCommands"
              :key="d.label"
              class="flex items-center gap-3"
            >
              <span class="text-[11px] text-[#888] font-mono w-24 shrink-0 text-right truncate" :title="d.label">{{ d.label }}</span>
              <div class="flex-1 flex items-center bg-[#252526] border border-[#333] rounded overflow-hidden">
                <code class="flex-1 text-xs text-[#ce9178] px-3 py-2 font-mono select-all break-all">{{ d.cmd }}</code>
                <button
                  @click="copyCmd(d.cmd)"
                  class="px-3 py-2 text-xs border-l border-[#333] shrink-0 cursor-pointer"
                  :class="copiedCmd === d.cmd ? 'text-green-400' : 'text-[#888] hover:text-white hover:bg-[#2a2d2e]'"
                >{{ copiedCmd === d.cmd ? 'Copied' : 'Copy' }}</button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Teleport>


    <!-- Save dialog -->
    <Teleport to="body">
      <div
        v-if="showSaveDialog"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click.self="showSaveDialog = false"
        @keydown.escape="showSaveDialog = false"
      >
        <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-md">
          <h2 class="text-sm font-semibold text-[#ddd] mb-4">Save {{ zone.name }}</h2>
          <input
            ref="commitInput"
            v-model="commitMsg"
            :placeholder="`Update ${zone.name}`"
            class="w-full bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-sm text-[#ccc] focus:border-blue-500 focus:outline-none placeholder-[#555]"
            @keydown.enter="save"
          />
          <label class="flex items-center gap-2 mt-3 cursor-pointer">
            <input type="checkbox" v-model="bumpSerial" class="accent-blue-500" />
            <span class="text-xs text-[#888]">Bump serial</span>
          </label>
          <div class="flex justify-end gap-2 mt-4">
            <button
              @click="showSaveDialog = false"
              class="px-3 py-1.5 text-xs rounded text-[#888] hover:text-white hover:bg-[#2a2d2e] cursor-pointer"
            >Cancel</button>
            <button
              @click="save"
              :disabled="saving"
              class="px-3 py-1.5 text-xs rounded bg-blue-600 hover:bg-blue-700 text-white disabled:opacity-50 cursor-pointer"
            >{{ saving ? 'Saving...' : 'Save' }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- Notify dialog -->
    <Teleport to="body">
      <div
        v-if="showNotifyDialog"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        @click.self="showNotifyDialog = false"
        @keydown.escape="showNotifyDialog = false"
      >
        <div class="bg-[#1e1e1e] border border-[#3e3e42] rounded-lg shadow-2xl p-6 w-full max-w-sm">
          <h2 class="text-sm font-semibold text-[#ddd] mb-4">Send NOTIFY for {{ zone.name }}</h2>
          <label class="text-xs text-[#888] mb-1 block">Target (ip:port)</label>
          <input
            ref="notifyInput"
            v-model="notifyTarget"
            placeholder="192.168.1.1:53"
            list="notify-target-options"
            class="w-full bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-sm text-[#ccc] focus:border-blue-500 focus:outline-none placeholder-[#555] font-mono"
            @keydown.enter="sendNotify"
          />
          <datalist id="notify-target-options">
            <option v-for="o in notifyAddresses" :key="o.identity" :value="o.address">{{ o.identity }}</option>
          </datalist>
          <div v-if="notifyStatus" class="mt-2 text-xs" :class="notifyStatus.ok ? 'text-green-400' : 'text-red-400'">
            {{ notifyStatus.message }}
          </div>
          <div class="flex justify-end gap-2 mt-4">
            <button
              @click="showNotifyDialog = false"
              class="px-3 py-1.5 text-xs rounded text-[#888] hover:text-white hover:bg-[#2a2d2e] cursor-pointer"
            >Close</button>
            <button
              @click="sendNotify"
              :disabled="notifySending || !notifyTarget.trim()"
              class="px-3 py-1.5 text-xs rounded bg-blue-600 hover:bg-blue-700 text-white disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
            >{{ notifySending ? 'Sending...' : 'Send' }}</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
