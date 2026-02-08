<script>
export default {
  props: {
    lines: { type: Array, required: true },
  },

  methods: {
    lineClass(line) {
      if (line.type === 'add') return 'diff-add'
      if (line.type === 'remove') return 'diff-remove'
      return ''
    },

    charClass(span) {
      if (span.type === 'add') return 'diff-char-add'
      if (span.type === 'remove') return 'diff-char-remove'
      return ''
    },
  },
}
</script>

<template>
  <div class="font-mono text-xs leading-5 overflow-auto">
    <div
      v-for="(line, i) in lines"
      :key="i"
      class="flex"
      :class="lineClass(line)"
    >
      <span class="w-8 text-right text-[#555] select-none shrink-0 px-1">{{ line.oldNum || '' }}</span>
      <span class="w-8 text-right text-[#555] select-none shrink-0 px-1 border-r border-[#333]">{{ line.newNum || '' }}</span>
      <span class="px-2 whitespace-pre flex-1 min-w-0">
        <template v-if="line.charDiff">
          <span
            v-for="(span, j) in line.charDiff"
            :key="j"
            :class="charClass(span)"
          >{{ span.text }}</span>
        </template>
        <template v-else>{{ line.content || ' ' }}</template>
      </span>
    </div>
  </div>
</template>
