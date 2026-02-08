export interface CharSpan {
  type: 'same' | 'add' | 'remove'
  text: string
}

export interface DiffLine {
  type: 'add' | 'remove' | 'context'
  oldNum?: number
  newNum?: number
  content: string
  charDiff?: CharSpan[]
}

function lcs(a: string[], b: string[]): boolean[][] {
  const m = a.length, n = b.length
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0))
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] = a[i - 1] === b[j - 1]
        ? dp[i - 1][j - 1] + 1
        : Math.max(dp[i - 1][j], dp[i][j - 1])
    }
  }

  // Backtrack to find which lines are common
  const inA = Array(m).fill(false)
  const inB = Array(n).fill(false)
  let i = m, j = n
  while (i > 0 && j > 0) {
    if (a[i - 1] === b[j - 1]) {
      inA[i - 1] = true
      inB[j - 1] = true
      i--; j--
    } else if (dp[i - 1][j] >= dp[i][j - 1]) {
      i--
    } else {
      j--
    }
  }
  return [inA, inB]
}

export function diffChars(oldStr: string, newStr: string): { old: CharSpan[]; new: CharSpan[] } {
  const oldChars = [...oldStr]
  const newChars = [...newStr]
  const m = oldChars.length, n = newChars.length
  const dp: number[][] = Array.from({ length: m + 1 }, () => Array(n + 1).fill(0))

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      dp[i][j] = oldChars[i - 1] === newChars[j - 1]
        ? dp[i - 1][j - 1] + 1
        : Math.max(dp[i - 1][j], dp[i][j - 1])
    }
  }

  // Backtrack
  const oldSpans: CharSpan[] = []
  const newSpans: CharSpan[] = []
  let i = m, j = n

  const oldOps: { type: 'same' | 'remove'; ch: string }[] = []
  const newOps: { type: 'same' | 'add'; ch: string }[] = []

  while (i > 0 || j > 0) {
    if (i > 0 && j > 0 && oldChars[i - 1] === newChars[j - 1]) {
      oldOps.push({ type: 'same', ch: oldChars[i - 1] })
      newOps.push({ type: 'same', ch: newChars[j - 1] })
      i--; j--
    } else if (i > 0 && (j === 0 || dp[i - 1][j] >= dp[i][j - 1])) {
      oldOps.push({ type: 'remove', ch: oldChars[i - 1] })
      i--
    } else {
      newOps.push({ type: 'add', ch: newChars[j - 1] })
      j--
    }
  }

  oldOps.reverse()
  newOps.reverse()

  // Merge consecutive spans of same type
  for (const op of oldOps) {
    if (oldSpans.length && oldSpans[oldSpans.length - 1].type === op.type) {
      oldSpans[oldSpans.length - 1].text += op.ch
    } else {
      oldSpans.push({ type: op.type, text: op.ch })
    }
  }
  for (const op of newOps) {
    if (newSpans.length && newSpans[newSpans.length - 1].type === op.type) {
      newSpans[newSpans.length - 1].text += op.ch
    } else {
      newSpans.push({ type: op.type, text: op.ch })
    }
  }

  return { old: oldSpans, new: newSpans }
}

export function diffLines(oldText: string, newText: string): DiffLine[] {
  if (oldText === newText) {
    return oldText.split('\n').map((content, i) => ({
      type: 'context' as const, oldNum: i + 1, newNum: i + 1, content,
    }))
  }
  const oldLines = oldText === '' ? [] : oldText.split('\n')
  const newLines = newText === '' ? [] : newText.split('\n')
  const [inOld, inNew] = lcs(oldLines, newLines)

  const result: DiffLine[] = []
  let oi = 0, ni = 0, oldNum = 1, newNum = 1

  while (oi < oldLines.length || ni < newLines.length) {
    if (oi < oldLines.length && inOld[oi] && ni < newLines.length && inNew[ni]) {
      // Context line (same in both)
      result.push({ type: 'context', oldNum: oldNum++, newNum: newNum++, content: oldLines[oi] })
      oi++; ni++
    } else {
      // Collect consecutive removes and adds
      const removes: { line: string; num: number }[] = []
      const adds: { line: string; num: number }[] = []

      while (oi < oldLines.length && !inOld[oi]) {
        removes.push({ line: oldLines[oi], num: oldNum++ })
        oi++
      }
      while (ni < newLines.length && !inNew[ni]) {
        adds.push({ line: newLines[ni], num: newNum++ })
        ni++
      }

      // Pair up removes/adds for char-level diff
      const pairCount = Math.min(removes.length, adds.length)
      for (let p = 0; p < pairCount; p++) {
        const chars = diffChars(removes[p].line, adds[p].line)
        result.push({ type: 'remove', oldNum: removes[p].num, content: removes[p].line, charDiff: chars.old })
        result.push({ type: 'add', newNum: adds[p].num, content: adds[p].line, charDiff: chars.new })
      }

      // Remaining unpaired removes
      for (let p = pairCount; p < removes.length; p++) {
        result.push({ type: 'remove', oldNum: removes[p].num, content: removes[p].line })
      }
      // Remaining unpaired adds
      for (let p = pairCount; p < adds.length; p++) {
        result.push({ type: 'add', newNum: adds[p].num, content: adds[p].line })
      }
    }
  }

  return result
}
