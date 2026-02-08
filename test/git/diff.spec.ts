import { describe, it, expect } from 'vitest'
import { diffLines, diffChars } from '../../src/git/diff'

describe('diffLines', () => {
  it('returns empty for identical text', () => {
    const result = diffLines('a\nb\nc', 'a\nb\nc')
    expect(result.every(l => l.type === 'context')).toBe(true)
    expect(result.length).toBe(3)
  })

  it('detects added lines', () => {
    const result = diffLines('a\nc', 'a\nb\nc')
    const types = result.map(l => l.type)
    expect(types).toEqual(['context', 'add', 'context'])
    expect(result[1].content).toBe('b')
  })

  it('detects removed lines', () => {
    const result = diffLines('a\nb\nc', 'a\nc')
    const types = result.map(l => l.type)
    expect(types).toEqual(['context', 'remove', 'context'])
    expect(result[1].content).toBe('b')
  })

  it('detects modified lines with char-level diff', () => {
    const result = diffLines(
      ';; Exported:  2022-05-15 01:30:45',
      ';; Exported:  2022-05-15 01:33:20',
    )
    expect(result.length).toBe(2)
    expect(result[0].type).toBe('remove')
    expect(result[1].type).toBe('add')
    expect(result[0].charDiff).toBeDefined()
    expect(result[1].charDiff).toBeDefined()
  })

  it('handles SOA serial change scenario', () => {
    const old = [
      ';; SOA Record',
      'backupmydns.com 3600  IN  SOA  backupmydns.com root.backupmydns.com 2040424484 7200 3600 86400 3600',
    ].join('\n')
    const updated = [
      ';; SOA Record',
      'backupmydns.com 3600  IN  SOA  backupmydns.com root.backupmydns.com 2040424500 7200 3600 86400 3600',
    ].join('\n')

    const result = diffLines(old, updated)
    const context = result.filter(l => l.type === 'context')
    const removes = result.filter(l => l.type === 'remove')
    const adds = result.filter(l => l.type === 'add')

    expect(context.length).toBe(1)
    expect(context[0].content).toBe(';; SOA Record')
    expect(removes.length).toBe(1)
    expect(adds.length).toBe(1)

    // The char diff should highlight the serial number change
    const removeSpans = removes[0].charDiff!
    const changedRemove = removeSpans.filter(s => s.type === 'remove')
    expect(changedRemove.length).toBeGreaterThan(0)
  })

  it('handles empty old text (all additions)', () => {
    const result = diffLines('', 'a\nb')
    expect(result.every(l => l.type === 'add')).toBe(true)
  })

  it('handles empty new text (all removals)', () => {
    const result = diffLines('a\nb', '')
    expect(result.every(l => l.type === 'remove')).toBe(true)
  })

  it('assigns correct line numbers', () => {
    const result = diffLines('a\nb\nc', 'a\nx\nc')
    const remove = result.find(l => l.type === 'remove')!
    const add = result.find(l => l.type === 'add')!
    expect(remove.oldNum).toBe(2)
    expect(add.newNum).toBe(2)
  })
})

describe('diffChars', () => {
  it('highlights changed characters', () => {
    const { old: oldSpans, new: newSpans } = diffChars('hello world', 'hello earth')
    const changed = oldSpans.filter(s => s.type === 'remove')
    expect(changed.length).toBeGreaterThan(0)
    const added = newSpans.filter(s => s.type === 'add')
    expect(added.length).toBeGreaterThan(0)
  })

  it('returns all same for identical strings', () => {
    const { old: oldSpans } = diffChars('same', 'same')
    expect(oldSpans.every(s => s.type === 'same')).toBe(true)
  })

  it('handles serial number change', () => {
    const { old: oldSpans, new: newSpans } = diffChars('2040424484', '2040424500')
    const removed = oldSpans.filter(s => s.type === 'remove').map(s => s.text).join('')
    const added = newSpans.filter(s => s.type === 'add').map(s => s.text).join('')
    expect(removed).toBeTruthy()
    expect(added).toBeTruthy()
  })
})
