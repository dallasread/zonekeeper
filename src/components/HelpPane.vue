<script>
export default {
  props: {
    app: { type: Object, required: true },
    serverRunning: { type: Boolean, required: true },
    zoneCount: { type: Number, required: true },
  },
}
</script>

<template>
  <div class="flex-1 overflow-y-auto">
  <div class="p-8 max-w-2xl mx-auto">
    <h1 class="text-lg font-semibold mb-6 text-[#ddd]">Zonekeeper</h1>

    <!-- Getting started -->
    <section v-if="!zoneCount" class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">Getting Started</h2>
      <ol class="space-y-2 text-sm text-[#aaa]">
        <li class="flex gap-2">
          <span class="text-[#666] shrink-0">1.</span>
          <span>Click <strong class="text-[#ddd]">+</strong> in the sidebar to add a zone (e.g. <code class="text-blue-400">test.local</code>)</span>
        </li>
        <li class="flex gap-2">
          <span class="text-[#666] shrink-0">2.</span>
          <span>Edit the zone file — add A, CNAME, MX, or any record type</span>
        </li>
        <li class="flex gap-2">
          <span class="text-[#666] shrink-0">3.</span>
          <span>Click <strong class="text-[#ddd]">Start</strong> to launch the server on port {{ app.port }}</span>
        </li>
        <li class="flex gap-2">
          <span class="text-[#666] shrink-0">4.</span>
          <span>Query your zone with <code class="text-blue-400">dig</code></span>
        </li>
      </ol>
    </section>

    <!-- Quick actions -->
    <section v-if="zoneCount" class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">Quick Start</h2>
      <p class="text-sm text-[#aaa]">Select a zone from the sidebar to edit it.</p>
    </section>

    <!-- Keyboard shortcuts -->
    <section class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">Keyboard Shortcuts</h2>
      <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        <kbd class="text-xs text-[#ddd] bg-[#252526] border border-[#3e3e42] rounded px-1.5 py-0.5 font-mono">&#8984;S</kbd>
        <span class="text-[#aaa]">Save zone file</span>
        <kbd class="text-xs text-[#ddd] bg-[#252526] border border-[#3e3e42] rounded px-1.5 py-0.5 font-mono">Tab</kbd>
        <span class="text-[#aaa]">Insert indent</span>
      </div>
    </section>

    <!-- Validation -->
    <section class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">Inline Validation</h2>
      <p class="text-sm text-[#aaa] mb-2">The editor catches errors as you type:</p>
      <div class="space-y-2 text-sm">
        <div class="flex items-start gap-2">
          <span class="shrink-0 text-xs bg-red-900/60 text-red-300 px-1.5 rounded mt-0.5">CNAME</span>
          <span class="text-[#aaa]">Invalid record types show the closest match — click to fix</span>
        </div>
        <div class="flex items-start gap-2">
          <span class="shrink-0 text-xs bg-red-900/60 text-red-300 px-1.5 rounded mt-0.5">3600</span>
          <span class="text-[#aaa]">Missing TTL — click to insert a default value</span>
        </div>
      </div>
    </section>

    <!-- Zone file format -->
    <section class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">Zone File Format</h2>
      <pre class="bg-[#252526] border border-[#3e3e42] rounded px-3 py-2 text-xs text-[#aaa] overflow-x-auto leading-5">$TTL 3600
@  IN  SOA  ns1.example.com. admin.example.com. (
    1       ; Serial
    3600    ; Refresh
    900     ; Retry
    604800  ; Expire
    86400   ; Minimum TTL
)

@       3600  IN  NS    ns1.example.com.
ns1     3600  IN  A     127.0.0.1
www     3600  IN  A     192.168.1.10
mail    3600  IN  CNAME www.example.com.</pre>
    </section>

    <!-- Record types -->
    <section class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">Supported Record Types</h2>
      <div class="flex flex-wrap gap-1.5">
        <span
          v-for="type in ['A', 'AAAA', 'CNAME', 'MX', 'NS', 'TXT', 'SOA', 'SRV', 'CAA', 'PTR', 'DNSKEY', 'DS', 'NAPTR', 'SSHFP', 'TLSA', 'SPF', 'HTTPS', 'SVCB', 'DNAME']"
          :key="type"
          class="text-xs bg-[#252526] border border-[#3e3e42] rounded px-1.5 py-0.5 text-[#aaa] font-mono"
        >{{ type }}</span>
      </div>
    </section>

    <!-- Architecture note -->
    <section class="mb-8">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-[#888] mb-3">How It Works</h2>
      <p class="text-sm text-[#aaa] leading-relaxed">
        Zonekeeper runs a DNS server on port <strong class="text-[#ddd]">{{ app.port }}</strong>.
        Zone files are stored in <code class="text-blue-400 text-xs">~/Library/Application Support/zonekeeper/</code>.
        The server is configured automatically from your zones and supports
        zone transfers (AXFR), so secondary servers can pull updates.
      </p>
    </section>

  </div>
  </div>
</template>
