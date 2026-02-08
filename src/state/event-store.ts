import { reactive } from 'vue'

export interface EventStoreEvent {
  collection: string
  objectId: string
  action: string
  data: any
  time: number
}

type Runner = (this: Record<string, any[]>, event: EventStoreEvent) => void

export interface Runners {
  [key: string]: Runner
}

export default class EventStore {
  _state: Record<string, any[]>
  _runners: Runners

  static RUNNERS = {
    CREATE(this: Record<string, any[]>, event: EventStoreEvent) {
      this[event.collection].push({
        ...event.data,
        id: event.objectId,
        createdAt: event.time,
        _collection: event.collection,
      })
    },

    UPDATE(this: Record<string, any[]>, event: EventStoreEvent) {
      const existing = this[event.collection].find(
        (item: any) => item.id === event.objectId
      )

      if (!existing) {
        return EventStore.RUNNERS.CREATE.call(this, event)
      }

      existing.updatedAt = event.time
      Object.assign(existing, event.data)
    },

    DELETE(this: Record<string, any[]>, event: EventStoreEvent) {
      const existing = this[event.collection].find(
        (item: any) => item.id === event.objectId
      )
      if (existing) {
        existing._deleted = true
        existing.deletedAt = event.time
      }
    },
  }

  constructor(runners: Runners) {
    this._runners = runners
    this._state = reactive({
      zones: [],
      logs: [],
    })
  }

  track(
    collection: string,
    objectId: string | null,
    action: string,
    data?: any
  ) {
    if (!objectId) {
      objectId = crypto.randomUUID()
    }

    const event: EventStoreEvent = {
      collection,
      objectId,
      action,
      data: data || {},
      time: Date.now(),
    }

    this._runEvent(event)
    return event
  }

  clear(collection: string) {
    if (this._state[collection]) {
      this._state[collection].length = 0
    }
  }

  findAll(collection: string) {
    const items = this._state[collection] || []
    return items.filter((item: any) => !item._deleted)
  }

  findAllWithDeleted(collection: string) {
    return this._state[collection] || []
  }

  _runEvent(event: EventStoreEvent) {
    const runnerKey = `${event.collection}.${event.action}`
    const runner = this._runners[runnerKey]
    if (runner) {
      runner.call(this._state, event)
    }
  }
}
