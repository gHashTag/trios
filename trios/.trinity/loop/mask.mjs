/**
 * mask.mjs - blank out the inside of every comment and string literal.
 *
 * WHY THIS IS ITS OWN FILE.
 *
 * This walk was written inside `cycle-doctor.mjs` on 2026-09-12, to stop a
 * write-detector reading a quoted example in a selftest fixture as a real call
 * to `appendFileSync`. Its own comment there said: "if a third caller needs this
 * walk, the two should become one generator". Within the hour `selftest.mjs`
 * became the third caller - because its import-graph scanner had read the words
 * `import('./cycle.mjs')` out of a PROSE COMMENT and reported cycle.mjs as
 * imported by selftest.mjs, which is how a gate comes to accuse a file on the
 * strength of a sentence describing it.
 *
 * That is the same defect the walk exists to prevent, committed by the file
 * that prevents it. So it moved here instead of being copied a third time.
 *
 * THE CONTRACT: the return value is the SAME LENGTH as the input, and newlines
 * are preserved. A caller finds a construct in the mask - where it is certain
 * to be code and not prose - and then slices the real text out of the ORIGINAL
 * at the same offsets. Quotes and comment markers themselves are kept, so the
 * offsets of surrounding code read naturally when printed.
 *
 * WHAT IT DOES NOT DO: it is a lexer, not a parser. A regex literal containing
 * a quote (`/['"]/`) will open a string that never closes on that line, and the
 * rest of the line is blanked. That is the safe direction - a caller sees less
 * code, never more prose - but it means a mask is evidence for accusing and not
 * for acquitting.
 */

export function maskLiterals(src) {
  const out = src.split('')
  let i = 0
  const n = src.length
  const blank = (from, to) => { for (let k = from; k < to && k < n; k++) if (out[k] !== '\n') out[k] = ' ' }
  while (i < n) {
    const c = src[i]
    if (c === '/' && src[i + 1] === '/') {
      const start = i
      while (i < n && src[i] !== '\n') i++
      blank(start, i)
    } else if (c === '/' && src[i + 1] === '*') {
      const start = i
      i += 2
      while (i < n && !(src[i] === '*' && src[i + 1] === '/')) i++
      i += 2
      blank(start, i)
    } else if (c === "'" || c === '"' || c === '`') {
      const quote = c
      const start = ++i
      while (i < n) {
        if (src[i] === '\\') { i += 2; continue }
        if (src[i] === quote) break
        if (quote !== '`' && src[i] === '\n') break
        i++
      }
      blank(start, i)
      i++
    } else {
      i++
    }
  }
  return out.join('')
}

const isMain = process.argv[1] && process.argv[1].endsWith('/mask.mjs')
if (isMain) {
  // Nothing to measure here - this file holds one pure function. Running it
  // prints the mask of a file so a reader can see what a detector sees.
  const file = process.argv[2]
  if (!file) {
    process.stdout.write('usage: node mask.mjs <file>   # prints what a detector sees: code, no prose\n')
    process.exit(1)
  }
  const fs = await import('node:fs')
  process.stdout.write(maskLiterals(fs.readFileSync(file, 'utf8')))
}
