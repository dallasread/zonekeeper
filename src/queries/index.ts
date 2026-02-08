import EventStore from '../state/event-store'

export default class Queries {
  state: EventStore

  constructor({ state }: { state: EventStore }) {
    this.state = state
  }

  allZones() {
    return this.state
      .findAll('zones')
      .sort((a: any, b: any) => a.name.localeCompare(b.name))
  }

  findZone(id: string) {
    return this.state.findAll('zones').find((zone: any) => zone.id === id)
  }

  allLogs() {
    return this.state
      .findAll('logs')
      .sort((a: any, b: any) => a.createdAt - b.createdAt)
  }
}
