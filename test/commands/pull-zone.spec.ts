import { describe, it, expect, beforeEach, vi } from 'vitest'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn(),
}))

vi.mock('../../src/git/service', () => ({
  ensureInitialCommit: vi.fn(),
  commitZone: vi.fn(),
  commitDelete: vi.fn(),
  readBlobAtCommit: vi.fn(),
}))

import { invoke } from '@tauri-apps/api/core'
import { emit } from '@tauri-apps/api/event'
import { commitZone } from '../../src/git/service'
import Commands from '../../src/commands'
import EventStore from '../../src/state/event-store'
import Queries from '../../src/queries'
import runners from '../../src/state/runners'

const mockedInvoke = vi.mocked(invoke)
const mockedEmit = vi.mocked(emit)
const mockedCommitZone = vi.mocked(commitZone)

let commands: Commands
let state: EventStore
let queries: Queries

beforeEach(() => {
  vi.resetAllMocks()
  state = new EventStore(runners)
  queries = new Queries({ state })
  commands = new Commands({
    state,
    queries,
    getPort: () => 1053,
    getIdentity: () => 'test-identity',
    getIdentityName: () => 'Test Server',
  })

  // Seed a zone in state
  state.track('zones', 'example.com', 'create', {
    name: 'example.com',
    content: 'old content',
  })
})

describe('pullZone', () => {
  it('invokes pull_zone with correct params', async () => {
    mockedInvoke.mockResolvedValue('new content\n')
    await commands.pullZone({ id: 'example.com' })

    expect(mockedInvoke).toHaveBeenCalledWith('pull_zone', {
      identity: 'test-identity',
      name: 'example.com',
      port: 1053,
    })
  })

  it('commits to git history with default Pull message', async () => {
    mockedInvoke.mockResolvedValue('new content\n')
    await commands.pullZone({ id: 'example.com' })

    expect(mockedCommitZone).toHaveBeenCalledWith(
      'test-identity',
      'example.com',
      'Pull example.com',
      'new content\n',
    )
  })

  it('commits to git history with custom message', async () => {
    mockedInvoke.mockResolvedValue('synced content\n')
    await commands.pullZone({ id: 'example.com' }, 'Sync example.com')

    expect(mockedCommitZone).toHaveBeenCalledWith(
      'test-identity',
      'example.com',
      'Sync example.com',
      'synced content\n',
    )
  })

  it('updates zone content in state', async () => {
    mockedInvoke.mockResolvedValue('pulled content\n')
    await commands.pullZone({ id: 'example.com' })

    const zone = queries.findZone('example.com')
    expect(zone.content).toBe('pulled content\n')
  })

  it('emits zones-changed event', async () => {
    mockedInvoke.mockResolvedValue('content\n')
    await commands.pullZone({ id: 'example.com' })

    expect(mockedEmit).toHaveBeenCalledWith('zones-changed-test-identity')
  })

  it('returns the pulled content', async () => {
    mockedInvoke.mockResolvedValue('returned content\n')
    const result = await commands.pullZone({ id: 'example.com' })

    expect(result).toBe('returned content\n')
  })

  it('still updates state if git commit fails', async () => {
    mockedInvoke.mockResolvedValue('content after git fail\n')
    mockedCommitZone.mockRejectedValue(new Error('git broken'))

    await commands.pullZone({ id: 'example.com' })

    const zone = queries.findZone('example.com')
    expect(zone.content).toBe('content after git fail\n')
  })

  it('creates zone in state if it did not exist', async () => {
    mockedInvoke.mockResolvedValue('new zone content\n')
    await commands.pullZone({ id: 'brand-new.com' })

    const zone = queries.findZone('brand-new.com')
    expect(zone).toBeDefined()
    expect(zone.content).toBe('new zone content\n')
  })
})

describe('pullZoneFor', () => {
  it('invokes pull_zone with explicit identity and port', async () => {
    mockedInvoke.mockResolvedValue('content\n')
    await commands.pullZoneFor('other-identity', 1054, 'example.com', 'Sync example.com')

    expect(mockedInvoke).toHaveBeenCalledWith('pull_zone', {
      identity: 'other-identity',
      name: 'example.com',
      port: 1054,
    })
  })

  it('commits with the given message', async () => {
    mockedInvoke.mockResolvedValue('synced\n')
    await commands.pullZoneFor('other-identity', 1054, 'example.com', 'Sync example.com')

    expect(mockedCommitZone).toHaveBeenCalledWith(
      'other-identity',
      'example.com',
      'Sync example.com',
      'synced\n',
    )
  })

  it('updates state when identity matches current', async () => {
    mockedInvoke.mockResolvedValue('synced\n')
    await commands.pullZoneFor('test-identity', 1053, 'example.com', 'Sync example.com')

    const zone = queries.findZone('example.com')
    expect(zone.content).toBe('synced\n')
  })

  it('does not update state when identity differs', async () => {
    mockedInvoke.mockResolvedValue('synced\n')
    await commands.pullZoneFor('other-identity', 1054, 'example.com', 'Sync example.com')

    const zone = queries.findZone('example.com')
    expect(zone.content).toBe('old content')
  })
})
