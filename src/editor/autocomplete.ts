import { VALID_TYPES_ARRAY, VALID_TYPES, CLASSES } from './validation'

// Record types that expect a hostname as rdata (or as last rdata token for MX/SRV)
const HOSTNAME_RDATA_TYPES = new Set(['CNAME', 'NS', 'PTR', 'DNAME'])
const HOSTNAME_AFTER_INT_TYPES = new Set(['MX']) // priority then hostname
const HOSTNAME_LAST_TYPES = new Set(['SRV'])     // priority weight port target

export interface AutocompleteContext {
  type: 'record-type' | 'hostname'
  prefix: string
  startPos: number // absolute position in content where the prefix starts
}

export function getContext(content: string, cursorPos: number): AutocompleteContext | null {
  // Find the current line
  const before = content.slice(0, cursorPos)
  const lineStart = before.lastIndexOf('\n') + 1
  const lineText = before.slice(lineStart)

  // Skip comments, directives, empty
  const trimmed = lineText.trimStart()
  if (!trimmed || trimmed.startsWith(';') || trimmed.startsWith('$')) return null

  // Get the current word being typed (last non-whitespace segment)
  const match = lineText.match(/(\S+)$/)
  const prefix = match ? match[1] : ''
  const startPos = cursorPos - prefix.length

  // If there's no prefix yet and we just typed whitespace, determine context from position
  // Tokenize the line up to cursor to figure out what field we're in
  const withoutComment = lineText.replace(/\s*;.*$/, '')
  const tokens = withoutComment.split(/\s+/).filter(Boolean)

  // Remove the partial token (the prefix) from the token list for position counting
  // unless prefix is empty (cursor is after whitespace)
  const fullTokens = prefix ? tokens.slice(0, -1) : tokens

  let i = 0
  let hasType: string | null = null

  // Check if first token is a name (line doesn't start with whitespace)
  const startsWithName = !lineText.startsWith(' ') && !lineText.startsWith('\t')
  if (startsWithName && fullTokens.length > 0) {
    i = 1 // skip name token
  }

  // Scan through tokens to determine where we are
  while (i < fullTokens.length) {
    const upper = fullTokens[i].toUpperCase()

    // Skip TTL
    if (/^\d+[smhdw]?$/i.test(fullTokens[i])) {
      i++
      continue
    }

    // Skip class
    if (CLASSES.has(upper)) {
      i++
      continue
    }

    if (VALID_TYPES.has(upper)) {
      hasType = upper
      i++
      break
    }

    // Unknown token in a position where we expect type — this could be a partial type
    break
  }

  // If we haven't found a type yet, we're in the type position
  if (!hasType) {
    // Only suggest if we're past the name (and optionally TTL/class)
    if (prefix.length >= 1) {
      return { type: 'record-type', prefix, startPos }
    }
    return null
  }

  // We have a type — check if the rdata expects a hostname
  const rdataTokens = fullTokens.slice(i)

  if (HOSTNAME_RDATA_TYPES.has(hasType) && rdataTokens.length === 0) {
    return { type: 'hostname', prefix, startPos }
  }

  if (HOSTNAME_AFTER_INT_TYPES.has(hasType) && rdataTokens.length === 1) {
    // MX: after priority, expect hostname
    return { type: 'hostname', prefix, startPos }
  }

  if (HOSTNAME_LAST_TYPES.has(hasType) && rdataTokens.length === 3) {
    // SRV: after priority weight port, expect target
    return { type: 'hostname', prefix, startPos }
  }

  return null
}

export function getSuggestions(ctx: AutocompleteContext, zoneHostnames: string[]): string[] {
  const upper = ctx.prefix.toUpperCase()

  if (ctx.type === 'record-type') {
    return VALID_TYPES_ARRAY.filter(t => t.startsWith(upper))
  }

  if (ctx.type === 'hostname') {
    if (!ctx.prefix) return zoneHostnames.slice(0, 10)
    const lower = ctx.prefix.toLowerCase()
    return zoneHostnames.filter(h => h.toLowerCase().startsWith(lower))
  }

  return []
}
