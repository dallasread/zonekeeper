import { describe, it, expect } from 'vitest'
import { getContext, getSuggestions } from '../../src/editor/autocomplete'

describe('getContext', () => {
  it('returns record-type context after name TTL class', () => {
    const content = 'www  3600  IN  C'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('record-type')
    expect(ctx!.prefix).toBe('C')
  })

  it('returns record-type context after name TTL', () => {
    const content = 'www  3600  A'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('record-type')
    expect(ctx!.prefix).toBe('A')
  })

  it('returns record-type context for indented line after TTL class', () => {
    const content = '  3600  IN  M'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('record-type')
    expect(ctx!.prefix).toBe('M')
  })

  it('returns null for comments', () => {
    expect(getContext('; comment', 9)).toBeNull()
  })

  it('returns null for directives', () => {
    expect(getContext('$TTL 3600', 9)).toBeNull()
  })

  it('returns null for empty input', () => {
    expect(getContext('', 0)).toBeNull()
  })

  it('returns hostname context after CNAME type', () => {
    const content = 'alias  3600  IN  CNAME  tar'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('hostname')
    expect(ctx!.prefix).toBe('tar')
  })

  it('returns hostname context after MX priority', () => {
    const content = '@  3600  IN  MX  10  m'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('hostname')
    expect(ctx!.prefix).toBe('m')
  })

  it('returns hostname context after NS type', () => {
    const content = '@  3600  IN  NS  n'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('hostname')
    expect(ctx!.prefix).toBe('n')
  })

  it('returns hostname context after SRV priority weight port', () => {
    const content = '_http._tcp  3600  IN  SRV  10  0  443  t'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('hostname')
    expect(ctx!.prefix).toBe('t')
  })

  it('returns null after A type (not hostname rdata)', () => {
    const content = 'www  3600  IN  A  192'
    const ctx = getContext(content, content.length)
    expect(ctx).toBeNull()
  })

  it('handles multiline content with cursor on last line', () => {
    const content = 'www  3600  IN  A  1.2.3.4\nmail  3600  IN  CN'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.type).toBe('record-type')
    expect(ctx!.prefix).toBe('CN')
  })

  it('tracks correct startPos for prefix replacement', () => {
    const content = 'www  3600  IN  CN'
    const ctx = getContext(content, content.length)
    expect(ctx).not.toBeNull()
    expect(ctx!.startPos).toBe(content.length - 2) // "CN" starts 2 chars before end
  })
})

describe('getSuggestions', () => {
  it('filters record types by prefix', () => {
    const ctx = { type: 'record-type' as const, prefix: 'CN', startPos: 0 }
    const suggestions = getSuggestions(ctx, [])
    expect(suggestions).toEqual(['CNAME'])
  })

  it('returns multiple matching types', () => {
    const ctx = { type: 'record-type' as const, prefix: 'NS', startPos: 0 }
    const suggestions = getSuggestions(ctx, [])
    expect(suggestions).toContain('NS')
    expect(suggestions).toContain('NSEC')
    expect(suggestions).toContain('NSEC3')
    expect(suggestions).toContain('NSEC3PARAM')
  })

  it('is case insensitive for type matching', () => {
    const ctx = { type: 'record-type' as const, prefix: 'cn', startPos: 0 }
    const suggestions = getSuggestions(ctx, [])
    expect(suggestions).toEqual(['CNAME'])
  })

  it('filters hostnames by prefix', () => {
    const ctx = { type: 'hostname' as const, prefix: 'ma', startPos: 0 }
    const hosts = ['www', 'mail', 'ftp', 'mx']
    const suggestions = getSuggestions(ctx, hosts)
    expect(suggestions).toEqual(['mail'])
  })

  it('returns hostnames when prefix is empty', () => {
    const ctx = { type: 'hostname' as const, prefix: '', startPos: 0 }
    const hosts = ['www', 'mail']
    const suggestions = getSuggestions(ctx, hosts)
    expect(suggestions).toEqual(['www', 'mail'])
  })

  it('returns empty for no matches', () => {
    const ctx = { type: 'record-type' as const, prefix: 'ZZZ', startPos: 0 }
    expect(getSuggestions(ctx, [])).toEqual([])
  })
})
