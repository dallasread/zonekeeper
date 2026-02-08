import LightningFS from '@isomorphic-git/lightning-fs'

const cache: Record<string, LightningFS> = {}

export function createFs(identity: string): LightningFS {
  if (!cache[identity]) {
    cache[identity] = new LightningFS('zonekeeper-' + identity)
  }
  return cache[identity]
}
