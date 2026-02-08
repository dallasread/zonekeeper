import { describe, it, expect } from 'vitest'
import { validateLine, validateContent } from '../../src/editor/validation'

describe('validateLine', () => {
  // Existing behavior
  it('returns null for valid A record', () => {
    expect(validateLine('www  3600  IN  A  192.168.1.1', 1)).toBeNull()
  })

  it('returns null for valid record starting with @', () => {
    expect(validateLine('@  3600  IN  A  10.0.0.1', 1)).toBeNull()
  })

  it('returns null for comments', () => {
    expect(validateLine('; this is a comment', 1)).toBeNull()
  })

  it('returns null for directives', () => {
    expect(validateLine('$TTL 3600', 1)).toBeNull()
  })

  it('returns null for empty lines', () => {
    expect(validateLine('', 1)).toBeNull()
    expect(validateLine('   ', 1)).toBeNull()
  })

  it('detects invalid record type', () => {
    const err = validateLine('www  3600  IN  CAME  target.', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/Invalid record type/)
    expect(err!.fixLabel).toBe('CNAME')
  })

  it('detects missing TTL', () => {
    const err = validateLine('www  IN  A  1.2.3.4', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toBe('Missing TTL')
    expect(err!.fixLabel).toBe('3600')
  })

  // Rdata validation — A records
  it('detects invalid IPv4 in A record', () => {
    const err = validateLine('www  3600  IN  A  999.1.2.3', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/A record.*invalid IPv4/)
  })

  it('detects missing rdata in A record', () => {
    const err = validateLine('www  3600  IN  A', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/A record.*missing IPv4/)
  })

  it('accepts valid A record with all octets', () => {
    expect(validateLine('www  3600  IN  A  0.0.0.0', 1)).toBeNull()
    expect(validateLine('www  3600  IN  A  255.255.255.255', 1)).toBeNull()
  })

  // Rdata validation — AAAA records
  it('detects invalid IPv6 in AAAA record', () => {
    const err = validateLine('www  3600  IN  AAAA  not-an-ip', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/AAAA record.*invalid IPv6/)
  })

  it('accepts valid AAAA record', () => {
    expect(validateLine('www  3600  IN  AAAA  2001:db8::1', 1)).toBeNull()
    expect(validateLine('www  3600  IN  AAAA  ::1', 1)).toBeNull()
    expect(validateLine('www  3600  IN  AAAA  fe80::1', 1)).toBeNull()
  })

  // Rdata validation — CNAME records
  it('detects missing target in CNAME record', () => {
    const err = validateLine('alias  3600  IN  CNAME', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/CNAME record.*missing target/)
  })

  it('accepts valid CNAME record', () => {
    expect(validateLine('alias  3600  IN  CNAME  target.example.com.', 1)).toBeNull()
    expect(validateLine('alias  3600  IN  CNAME  @', 1)).toBeNull()
  })

  it('warns about missing trailing dot in CNAME', () => {
    const err = validateLine('alias  3600  IN  CNAME  target.example.com', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/missing trailing dot/)
    expect(err!.fixLabel).toBe('target.example.com.')
  })

  it('warns about missing trailing dot in NS', () => {
    const err = validateLine('@  3600  IN  NS  ns1.example.com', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/missing trailing dot/)
    expect(err!.fixLabel).toBe('ns1.example.com.')
  })

  it('warns about missing trailing dot in MX mail server', () => {
    const err = validateLine('@  3600  IN  MX  10  mail.example.com', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/missing trailing dot/)
    expect(err!.fixLabel).toBe('mail.example.com.')
  })

  it('does not warn about relative names without dots', () => {
    expect(validateLine('alias  3600  IN  CNAME  www', 1)).toBeNull()
  })

  it('fix appends trailing dot', () => {
    const err = validateLine('alias  3600  IN  CNAME  target.example.com', 1)
    expect(err!.fix).toBeDefined()
    const fixed = err!.fix!('alias  3600  IN  CNAME  target.example.com')
    expect(fixed).toBe('alias  3600  IN  CNAME  target.example.com.')
  })

  // Rdata validation — MX records
  it('detects missing priority in MX record', () => {
    const err = validateLine('@  3600  IN  MX', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/MX record.*requires priority/)
  })

  it('detects invalid MX priority', () => {
    const err = validateLine('@  3600  IN  MX  abc  mail.example.com.', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/MX record.*invalid priority/)
  })

  it('accepts valid MX record', () => {
    expect(validateLine('@  3600  IN  MX  10  mail.example.com.', 1)).toBeNull()
  })

  // Rdata validation — NS records
  it('detects missing NS target', () => {
    const err = validateLine('@  3600  IN  NS', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/NS record.*missing target/)
  })

  it('accepts valid NS record', () => {
    expect(validateLine('@  3600  IN  NS  ns1.example.com.', 1)).toBeNull()
  })

  // Rdata validation — SRV records
  it('detects incomplete SRV record', () => {
    const err = validateLine('_http._tcp  3600  IN  SRV  10  0', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/SRV record.*requires/)
  })

  it('detects invalid SRV port', () => {
    const err = validateLine('_http._tcp  3600  IN  SRV  10  0  99999  target.', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/SRV record.*invalid port/)
  })

  it('accepts valid SRV record', () => {
    expect(validateLine('_http._tcp  3600  IN  SRV  10  0  443  target.example.com.', 1)).toBeNull()
  })

  // Rdata validation — CAA records
  it('detects invalid CAA flags', () => {
    const err = validateLine('@  3600  IN  CAA  999  issue  "ca.com"', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/CAA record.*invalid flags/)
  })

  it('detects unknown CAA tag', () => {
    const err = validateLine('@  3600  IN  CAA  0  badtag  "ca.com"', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/CAA record.*unknown tag/)
  })

  it('accepts valid CAA record', () => {
    expect(validateLine('@  3600  IN  CAA  0  issue  "letsencrypt.org"', 1)).toBeNull()
    expect(validateLine('@  3600  IN  CAA  0  issuewild  ";"', 1)).toBeNull()
  })

  // Rdata validation — TXT records
  it('detects missing TXT data', () => {
    const err = validateLine('@  3600  IN  TXT', 1)
    expect(err).not.toBeNull()
    expect(err!.message).toMatch(/TXT record.*missing text/)
  })

  it('accepts valid TXT record', () => {
    expect(validateLine('@  3600  IN  TXT  "v=spf1 include:_spf.google.com ~all"', 1)).toBeNull()
  })

  // Rdata validation — PTR records
  it('accepts valid PTR record', () => {
    expect(validateLine('1  3600  IN  PTR  host.example.com.', 1)).toBeNull()
  })

  // Records without rdata validation pass through
  it('does not validate rdata for unknown types', () => {
    expect(validateLine('@  3600  IN  DNSKEY  257 3 13 abc123', 1)).toBeNull()
  })

  // Indented lines (continuation hostname)
  it('handles indented lines correctly', () => {
    expect(validateLine('  3600  IN  A  10.0.0.1', 1)).toBeNull()
  })

  // TTL shorthand
  it('handles TTL with time suffix', () => {
    expect(validateLine('www  1h  IN  A  10.0.0.1', 1)).toBeNull()
  })
})

describe('validateContent', () => {
  it('skips SOA block in parentheses', () => {
    const content = [
      '@  3600  IN  SOA  ns1.example.com.  admin.example.com. (',
      '  2024010100',
      '  7200',
      '  3600',
      '  86400',
      '  3600 )',
      'www  3600  IN  A  1.2.3.4',
    ].join('\n')
    const errors = validateContent(content)
    expect(errors).toEqual([])
  })

  it('reports multiple errors', () => {
    const content = [
      'www  3600  IN  A  999.0.0.1',
      'mail  3600  IN  MX',
    ].join('\n')
    const errors = validateContent(content)
    expect(errors.length).toBe(2)
  })
})
