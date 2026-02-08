import EventStore from './event-store'

export default {
  'zones.create': EventStore.RUNNERS.CREATE,
  'zones.update': EventStore.RUNNERS.UPDATE,
  'zones.delete': EventStore.RUNNERS.DELETE,
  'logs.create': EventStore.RUNNERS.CREATE,
}
