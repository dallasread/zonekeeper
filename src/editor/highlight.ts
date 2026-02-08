import { VALID_TYPES, CLASSES } from './validation'

export function esc(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}

export function highlightLine(line: string): string {
  if (!line) return ' '

  // Comments
  if (line.trimStart().startsWith(';')) {
    return `<span class="hl-comment">${esc(line)}</span>`
  }

  // Directives ($TTL, $ORIGIN, etc.)
  if (line.trimStart().startsWith('$')) {
    return `<span class="hl-directive">${esc(line)}</span>`
  }

  // Parentheses-only lines (SOA continuation)
  const trimmed = line.trim()
  if (/^[(\d\s;)]+$/.test(trimmed) || trimmed === '(' || trimmed === ')') {
    // Highlight inline comments within SOA blocks
    return line.replace(/^(.*?)(;.*)$/, (_, pre, comment) =>
      `${esc(pre)}<span class="hl-comment">${esc(comment)}</span>`
    ).replace(/^([^<]*)$/, (m) => esc(m))
  }

  // Record lines — tokenize and colorize
  let result = ''
  let remaining = line

  // Inline comment at end
  let comment = ''
  const commentMatch = remaining.match(/(\s*;.*)$/)
  if (commentMatch) {
    comment = commentMatch[1]
    remaining = remaining.slice(0, -comment.length)
  }

  const tokens = remaining.split(/(\s+)/)
  for (const token of tokens) {
    if (/^\s+$/.test(token)) {
      result += token
      continue
    }
    const upper = token.toUpperCase()
    if (VALID_TYPES.has(upper)) {
      result += `<span class="hl-type">${esc(token)}</span>`
    } else if (CLASSES.has(upper)) {
      result += `<span class="hl-class">${esc(token)}</span>`
    } else if (/^\d+[smhdw]?$/i.test(token)) {
      result += `<span class="hl-ttl">${esc(token)}</span>`
    } else if (/^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(token)) {
      result += `<span class="hl-value">${esc(token)}</span>`
    } else if (token === '@') {
      result += `<span class="hl-origin">${esc(token)}</span>`
    } else {
      result += esc(token)
    }
  }

  if (comment) {
    result += `<span class="hl-comment">${esc(comment)}</span>`
  }

  return result
}
