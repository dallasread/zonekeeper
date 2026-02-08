export const VALID_TYPES = new Set([
  'A', 'AAAA', 'CNAME', 'MX', 'NS', 'TXT', 'SOA', 'SRV', 'CAA',
  'PTR', 'DNSKEY', 'DS', 'NAPTR', 'SSHFP', 'TLSA', 'SPF', 'HINFO',
  'LOC', 'CERT', 'DNAME', 'AFSDB', 'CDNSKEY', 'CDS', 'DLV',
  'HTTPS', 'NSEC', 'NSEC3', 'NSEC3PARAM', 'RRSIG', 'SVCB', 'URI',
])

export const VALID_TYPES_ARRAY = [...VALID_TYPES]
export const CLASSES = new Set(['IN', 'CH', 'HS'])

export interface ValidationError {
  line: number
  message: string
  fixLabel?: string
  fix?: (line: string) => string
}

export function editDistance(a: string, b: string): number {
  const m = a.length
  const n = b.length
  const dp = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0))
  for (let i = 0; i <= m; i++) dp[i][0] = i
  for (let j = 0; j <= n; j++) dp[0][j] = j
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] = Math.min(
        dp[i - 1][j] + 1,
        dp[i][j - 1] + 1,
        dp[i - 1][j - 1] + (a[i - 1] !== b[j - 1] ? 1 : 0)
      )
    }
  }
  return dp[m][n]
}

export function closestType(invalid: string): string | null {
  const upper = invalid.toUpperCase()
  let best: string | null = null
  let bestDist = Infinity
  for (const type of VALID_TYPES_ARRAY) {
    const dist = editDistance(upper, type)
    if (dist < bestDist) {
      bestDist = dist
      best = type
    }
  }
  return best
}

export function fixInvalidType(line: string, invalidToken: string): string {
  const replacement = closestType(invalidToken)
  if (!replacement) return line
  return line.replace(new RegExp('\\b' + invalidToken + '\\b', 'i'), replacement)
}

export function fixMissingTTL(line: string): string {
  return line.replace(/(\s+)(IN|CH|HS)(\s)/i, '$1' + '3600  $2$3')
}

function isValidIPv4(s: string): boolean {
  const parts = s.split('.')
  if (parts.length !== 4) return false
  return parts.every(p => /^\d{1,3}$/.test(p) && Number(p) >= 0 && Number(p) <= 255)
}

function isValidIPv6(s: string): boolean {
  // Accept :: shorthand and hex groups separated by colons
  if (!s.includes(':')) return false
  const expanded = s.replace('::', ':PLACEHOLDER:')
  const groups = expanded.split(':').filter(Boolean)
  if (groups.length > 8) return false
  return groups.every(g => g === 'PLACEHOLDER' || /^[0-9a-fA-F]{1,4}$/.test(g))
}

function isHostname(s: string): boolean {
  if (s === '@' || s === '.') return true
  return /^[a-zA-Z0-9._*-]+$/.test(s)
}

// Hostname with dots but no trailing dot — likely missing FQDN qualifier
function missingTrailingDot(s: string): boolean {
  if (s === '@' || s === '.') return false
  return s.includes('.') && !s.endsWith('.')
}

function isPositiveInt(s: string): boolean {
  return /^\d+$/.test(s) && Number(s) >= 0
}

interface RdataResult {
  message?: string
  fixLabel?: string
  fixToken?: string  // the token to fix (append dot)
}

function checkHostnameDot(type: string, hostname: string, label: string): RdataResult | null {
  if (!isHostname(hostname)) return { message: `${type} record: invalid ${label} "${hostname}"` }
  if (missingTrailingDot(hostname)) return {
    message: `${type} record: "${hostname}" is missing trailing dot`,
    fixLabel: hostname + '.',
    fixToken: hostname,
  }
  return null
}

function validateRdata(type: string, rdata: string[]): RdataResult | null {
  switch (type) {
    case 'A':
      if (rdata.length < 1) return { message: 'A record: missing IPv4 address' }
      if (!isValidIPv4(rdata[0])) return { message: `A record: invalid IPv4 address "${rdata[0]}"` }
      return null

    case 'AAAA':
      if (rdata.length < 1) return { message: 'AAAA record: missing IPv6 address' }
      if (!isValidIPv6(rdata[0])) return { message: `AAAA record: invalid IPv6 address "${rdata[0]}"` }
      return null

    case 'CNAME':
    case 'NS':
    case 'PTR':
    case 'DNAME':
      if (rdata.length < 1) return { message: `${type} record: missing target hostname` }
      return checkHostnameDot(type, rdata[0], 'hostname')

    case 'MX':
      if (rdata.length < 2) return { message: 'MX record: requires priority and mail server' }
      if (!isPositiveInt(rdata[0])) return { message: `MX record: invalid priority "${rdata[0]}"` }
      return checkHostnameDot('MX', rdata[1], 'mail server')

    case 'SRV':
      if (rdata.length < 4) return { message: 'SRV record: requires priority weight port target' }
      if (!isPositiveInt(rdata[0])) return { message: `SRV record: invalid priority "${rdata[0]}"` }
      if (!isPositiveInt(rdata[1])) return { message: `SRV record: invalid weight "${rdata[1]}"` }
      if (!isPositiveInt(rdata[2]) || Number(rdata[2]) > 65535) return { message: `SRV record: invalid port "${rdata[2]}"` }
      return checkHostnameDot('SRV', rdata[3], 'target')

    case 'CAA':
      if (rdata.length < 3) return { message: 'CAA record: requires flags tag value' }
      if (!isPositiveInt(rdata[0]) || Number(rdata[0]) > 255) return { message: `CAA record: invalid flags "${rdata[0]}"` }
      if (!['issue', 'issuewild', 'iodef'].includes(rdata[1].toLowerCase()))
        return { message: `CAA record: unknown tag "${rdata[1]}" (expected issue, issuewild, or iodef)` }
      return null

    case 'TXT':
    case 'SPF':
      if (rdata.length < 1) return { message: `${type} record: missing text data` }
      return null

    default:
      return null
  }
}

export function validateLine(line: string, lineNumber: number): ValidationError | null {
  const trimmed = line.trim()

  if (!trimmed || trimmed.startsWith(';') || trimmed.startsWith('$')) return null
  if (trimmed === '(' || trimmed === ')') return null
  if (/^[\d\s();]+$/.test(trimmed)) return null

  const withoutComment = trimmed.replace(/\s*;.*$/, '')
  const tokens = withoutComment.split(/\s+/).filter(Boolean)
  if (tokens.length < 2) return null

  let i = 0
  let hasTTL = false
  let hasClass = false

  if (!line.startsWith(' ') && !line.startsWith('\t')) {
    i = 1
  }

  while (i < tokens.length) {
    const upper = tokens[i].toUpperCase()

    if (/^\d+[smhdw]?$/i.test(tokens[i])) {
      hasTTL = true
      i++
      continue
    }

    if (CLASSES.has(upper)) {
      hasClass = true
      i++
      continue
    }

    if (VALID_TYPES.has(upper)) {
      if (hasClass && !hasTTL) {
        return {
          line: lineNumber,
          message: 'Missing TTL',
          fixLabel: '3600',
          fix: fixMissingTTL,
        }
      }
      // Validate rdata (tokens after the type)
      const rdata = tokens.slice(i + 1)
      const rdataResult = validateRdata(upper, rdata)
      if (rdataResult?.message) {
        const err: ValidationError = { line: lineNumber, message: rdataResult.message }
        if (rdataResult.fixLabel && rdataResult.fixToken) {
          err.fixLabel = rdataResult.fixLabel
          const token = rdataResult.fixToken
          err.fix = (l) => l.replace(new RegExp('\\b' + token.replace(/\./g, '\\.') + '(?=[\\s;]|$)'), token + '.')
        }
        return err
      }
      return null
    }

    if (hasTTL || hasClass) {
      const suggestion = closestType(tokens[i])
      return {
        line: lineNumber,
        message: `Invalid record type: ${tokens[i]}`,
        fixLabel: suggestion || undefined,
        fix: (l) => fixInvalidType(l, tokens[i]),
      }
    }

    if (i === 0) {
      i++
      continue
    }

    const suggestion = closestType(tokens[i])
    return {
      line: lineNumber,
      message: `Invalid record type: ${tokens[i]}`,
      fixLabel: suggestion || undefined,
      fix: (l) => fixInvalidType(l, tokens[i]),
    }
  }

  return null
}

export function validateContent(content: string): ValidationError[] {
  const lines = content.split('\n')
  const errors: ValidationError[] = []
  let inParens = false

  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim()

    if (inParens) {
      if (trimmed.includes(')')) inParens = false
      continue
    }

    if (trimmed.includes('(') && !trimmed.includes(')')) {
      inParens = true
    }

    const error = validateLine(lines[i], i + 1)
    if (error) errors.push(error)
  }

  return errors
}
