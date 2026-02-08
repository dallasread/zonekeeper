import git from 'isomorphic-git'
import { createFs } from './fs-adapter'

const DIR = '/'
const AUTHOR = { name: 'Zonekeeper', email: 'zonekeeper@local' }

let commitLock = Promise.resolve()

function withLock<T>(fn: () => Promise<T>): Promise<T> {
  const result = commitLock.then(fn, fn)
  commitLock = result.then(() => {}, () => {})
  return result
}

export async function ensureInitialCommit(identity: string, zones: Array<{ name: string; content: string }>) {
  const fs = createFs(identity)
  await withLock(async () => {
    try {
      await fs.promises.stat(`${DIR}/.git`)
    } catch {
      await git.init({ fs, dir: DIR, defaultBranch: 'main' })
    }

    try {
      await git.log({ fs, dir: DIR, depth: 1 })
    } catch {
      let hasFiles = false
      for (const zone of zones) {
        const filepath = `${zone.name}.zone`
        await fs.promises.writeFile(`${DIR}/${filepath}`, zone.content, 'utf8')
        await git.add({ fs, dir: DIR, filepath })
        hasFiles = true
      }
      if (hasFiles) {
        await git.commit({
          fs, dir: DIR,
          message: 'Initial commit',
          author: AUTHOR,
        })
      }
    }
  })
}

export async function commitZone(identity: string, zoneName: string, action: string, content: string) {
  const fs = createFs(identity)
  await withLock(async () => {
    const filepath = `${zoneName}.zone`
    await fs.promises.writeFile(`${DIR}/${filepath}`, content, 'utf8')
    await git.add({ fs, dir: DIR, filepath })

    const matrix = await git.statusMatrix({ fs, dir: DIR, filepaths: [filepath] })
    const hasChanges = matrix.some(([, head, , stage]) => head !== stage)
    if (!hasChanges) return

    await git.commit({
      fs, dir: DIR,
      message: action,
      author: AUTHOR,
    })
  })
}

export async function commitDelete(identity: string, zoneName: string) {
  const fs = createFs(identity)
  await withLock(async () => {
    const filepath = `${zoneName}.zone`
    try { await fs.promises.unlink(`${DIR}/${filepath}`) } catch {}
    await git.remove({ fs, dir: DIR, filepath })
    await git.commit({
      fs, dir: DIR,
      message: `Delete ${zoneName}`,
      author: AUTHOR,
    })
  })
}

export async function zoneLog(identity: string, zoneName: string) {
  const fs = createFs(identity)
  const filepath = `${zoneName}.zone`
  return git.log({ fs, dir: DIR, filepath })
}

export async function fullLog(identity: string) {
  const fs = createFs(identity)
  return git.log({ fs, dir: DIR })
}

export async function readBlobAtCommit(identity: string, oid: string, zoneName: string): Promise<string> {
  const fs = createFs(identity)
  const filepath = `${zoneName}.zone`
  const { blob } = await git.readBlob({ fs, dir: DIR, oid, filepath })
  return new TextDecoder().decode(blob)
}
