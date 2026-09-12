#!/usr/bin/env node
// The status page, in Claude Code's visual language, regenerated every cycle.
//
// WHY A SECOND SURFACE AT ALL. There are already two ANSI dashboards here, and
// a third would need an argument. This one has a different job: the ANSI boxes
// are 74 columns wide and everything that will not fit gets abbreviated or
// dropped - that is what the label-width defect was, and it is structural, not a
// bug. A reading in `state/cycle-readings.jsonl` carries FOUR fields per metric
// (`v`, `src`, `note`, `at`) and the terminal renders one and a half of them.
// The provenance is measured and then thrown away at the last step.
//
// So: same readings, no new measurement, nothing re-derived. This file is a
// renderer and only a renderer. If a number is not in the reading it is not on
// the page, and no number here is computed from another number here.
//
// WHAT IT REFUSES TO DO.
//   - It never writes `DASHBOARD.txt`, `state.json` or `ledger.jsonl`. Those
//     belong to iterations 1-96 and `cycle-doctor` has a contract case for it.
//   - It never prints 0 for a fact that was not measured. `null` renders as a
//     dash, in dim, and says so on hover. A zero that means "nobody looked" is
//     this repository's most-repeated defect and it will not be introduced by
//     the thing that displays it.
//   - It does not decide anything is fine. Staleness is shown as an age, and
//     the exit code is non-zero when the reading it drew is older than a cycle.
//
// Usage:
//   node dash-cc.mjs               # render to DASHBOARD.html
//   node dash-cc.mjs --open        # render, then open it
//   node dash-cc.mjs --stdout      # write the HTML to stdout instead
//
// EXIT CODE. 2 when the newest reading is older than `--stale-min` (default 30).
// The page still renders - a stale dashboard that draws is more useful than no
// dashboard, PROVIDED it says how old it is, which it does, at the top, first.

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const DIR = path.dirname(fileURLToPath(import.meta.url))
const READINGS = path.join(DIR, 'state', 'cycle-readings.jsonl')
const OUT = path.join(DIR, 'DASHBOARD.html')

const argOf = (argv, flag, dflt) => {
  const i = argv.indexOf(flag)
  return i >= 0 && argv[i + 1] ? argv[i + 1] : dflt
}

export function newestReading(file = READINGS) {
  if (!fs.existsSync(file)) return null
  const lines = fs.readFileSync(file, 'utf8').split('\n').filter((l) => l.trim())
  for (let i = lines.length - 1; i >= 0; i--) {
    try { return JSON.parse(lines[i]) } catch { /* a torn line is not a reading */ }
  }
  return null
}

const esc = (s) => String(s)
  .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  .replace(/"/g, '&quot;')

export function humanAge(ms) {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return null
  const m = Math.round(ms / 60000)
  if (m < 60) return `${m}m`
  const h = Math.floor(m / 60)
  if (h < 24) return `${h}h ${m % 60}m`
  return `${Math.floor(h / 24)}d ${h % 24}h`
}

/**
 * A measured cell, or an honest dash.
 *
 * `v === null` is NOT MEASURED and renders as a dash. It is the whole reason
 * this function exists rather than a template literal: every place a value is
 * interpolated is a place a zero can be substituted for an absence, and there
 * are forty of them on this page.
 */
export function cell(m, fmt = (x) => x) {
  if (!m || m.v === null || m.v === undefined) {
    return { text: '—', measured: false, src: m ? m.src : null, note: m ? m.note : null, at: m ? m.at : null }
  }
  return { text: String(fmt(m.v)), measured: true, src: m.src || null, note: m.note || null, at: m.at || null }
}

function row(label, m, fmt, tone) {
  // `fmt || undefined`: most call sites pass `null` for "no formatting", and a
  // default parameter answers only to `undefined`. Without this the identity
  // formatter never applies and every plain number throws.
  const c = cell(m, fmt || undefined)
  const title = [c.src ? `source: ${c.src}` : null, c.note || null].filter(Boolean).join('\n')
  return `      <div class="row${c.measured ? '' : ' unmeasured'}">
        <span class="k">${esc(label)}</span>
        <span class="v ${tone || ''}" title="${esc(title)}">${esc(c.text)}</span>
        <span class="src">${esc(c.src || 'not measured')}</span>
      </div>`
}

/** A metric-shaped wrapper for a value pulled out of a composite reading. */
const at = (parent, v, note) => ({ v, src: parent ? parent.src : null, note: note || (parent ? parent.note : null), at: parent ? parent.at : null })

export function render(r, opts = {}) {
  const now = opts.now || Date.now()
  const readingAt = r && r.at ? Date.parse(r.at) : null
  const ageMs = readingAt ? now - readingAt : null
  const age = humanAge(ageMs)
  const staleMin = opts.staleMin ?? 30
  const stale = ageMs === null || ageMs > staleMin * 60000

  const L = (r && r.loop) || {}
  const q = r && r.queen ? r.queen : null
  const g = r && r.git ? r.git : null
  const d = r && r.disk ? r.disk : null
  const gates = r && r.gates ? r.gates : null
  const timers = r && r.timers ? r.timers : null
  const lock = r && r.lock ? r.lock : null
  const driver = r && r.driver ? r.driver : null

  const qv = q && q.v ? q.v : null
  const gv = g && g.v ? g.v : null
  const dv = d && d.v ? d.v : null
  const gatev = gates && gates.v ? gates.v : null
  const lockv = lock && lock.v ? lock.v : null
  const drv = driver && driver.v ? driver.v : null

  // `null` means the list could not be computed. `[]` means it was computed and
  // was empty. Collapsing those two into one empty array is how a broken
  // detector comes to look like a healthy loop, so they stay apart all the way
  // to the markup.
  const anomFailed = opts.anomalies === null
  const all = anomFailed ? [] : (opts.anomalies || [])
  const RANK = { blocker: 0, high: 1, medium: 2, low: 3 }
  const sorted = all.slice().sort((a, b) =>
    (RANK[String(a.severity || '').toLowerCase()] ?? 9) - (RANK[String(b.severity || '').toLowerCase()] ?? 9))
  const SHOW = 12
  const anomalies = sorted.slice(0, SHOW)
  const anomHidden = sorted.length - anomalies.length

  // A COUNTER THAT STOPPED MOVING IS NOT A STATUS.
  //
  // The first render of this page put `iteration 96` in a card headed "the loop
  // itself" - and directly underneath, its own blocker read "state.json presents
  // iteration 96 as the current one; it closed 151.5 hours ago and no iteration
  // has opened since". Both were on the page and both were true, and a reader
  // scanning the cards would still have come away believing iteration 96 was
  // live. Being literally correct somewhere on the page is not the standard.
  // The repair line of that very anomaly says what to do: make the dashboard say
  // STOPPED. Both numbers are already in the reading; this derives nothing new.
  const STOPPED_AFTER_H = 24
  const stopped = !!(L.staleHours && L.staleHours.v !== null && Number(L.staleHours.v) > STOPPED_AFTER_H)

  // The refusal breakdown, when the Queen reported one. Shares are computed from
  // two numbers in the SAME reading, which is the only arithmetic on this page.
  const refusals = qv && qv.refusals && typeof qv.refusals === 'object' ? qv.refusals : null
  const skipTotal = refusals
    ? Object.values(refusals).reduce((a, b) => a + (Number(b) || 0), 0)
    : null
  const refusalRows = refusals
    ? Object.entries(refusals)
        .sort((a, b) => (Number(b[1]) || 0) - (Number(a[1]) || 0))
        .map(([k, v]) => {
          const share = skipTotal ? (100 * Number(v)) / skipTotal : null
          return `        <div class="bar-row">
          <span class="bk">${esc(k)}</span>
          <span class="bar"><i style="width:${share === null ? 0 : share.toFixed(1)}%"></i></span>
          <span class="bv">${esc(v)}${share === null ? '' : ` <em>${share.toFixed(1)}%</em>`}</span>
        </div>`
        })
        .join('\n')
    : ''

  const timerRows = timers && timers.v
    ? Object.entries(timers.v).map(([name, t]) => {
        const mins = t && Number.isFinite(Number(t.lastWriteMin)) ? Number(t.lastWriteMin) : null
        const tone = mins === null ? '' : mins > 60 ? 'bad' : mins > 20 ? 'warn' : 'good'
        return `        <div class="chip ${tone}"><b>${esc(name)}</b><span>${mins === null ? '—' : `${mins}m ago`}</span></div>`
      }).join('\n')
    : ''

  return `<!doctype html>
<meta charset="utf-8">
<title>TRIOS — loop status</title>
<style>
  :root {
    --bg: #1f1e1d; --panel: #262624; --line: #3a3937;
    --ink: #f0eee6; --dim: #8f8b83; --faint: #6b6761;
    --accent: #d97757; --good: #7fb069; --warn: #d9a441; --bad: #d16d63;
    --mono: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, monospace;
  }
  * { box-sizing: border-box; }
  body { margin: 0; background: var(--bg); color: var(--ink); font-family: var(--mono);
         font-size: 13px; line-height: 1.55; padding: 28px 24px 48px; }
  .wrap { max-width: 1040px; margin: 0 auto; }
  h1 { font-size: 15px; font-weight: 600; margin: 0 0 2px; letter-spacing: .02em; }
  h1 .dot { color: var(--accent); }
  .sub { color: var(--dim); font-size: 12px; margin-bottom: 4px; }
  .banner { margin: 14px 0 22px; padding: 9px 13px; border-radius: 6px;
            border: 1px solid var(--line); background: var(--panel); color: var(--dim); font-size: 12px; }
  .banner.stale { border-color: #6d4a3f; background: #2e2320; color: #e7b9a6; }
  .banner b { color: var(--ink); font-weight: 600; }
  .grid { display: grid; grid-template-columns: repeat(2, minmax(0,1fr)); gap: 14px; }
  .card { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; padding: 13px 15px 15px; }
  .card.full { grid-column: 1 / -1; }
  .card h2 { font-size: 11px; text-transform: uppercase; letter-spacing: .09em;
             color: var(--faint); margin: 0 0 10px; font-weight: 600; }
  .row { display: grid; grid-template-columns: 1fr auto; grid-template-areas: "k v" "src src";
         gap: 0 10px; padding: 4px 0; border-bottom: 1px solid rgba(255,255,255,.03); }
  .row:last-child { border-bottom: 0; }
  .k { grid-area: k; color: var(--dim); }
  .v { grid-area: v; color: var(--ink); font-weight: 600; font-variant-numeric: tabular-nums; }
  .v.good { color: var(--good); } .v.warn { color: var(--warn); } .v.bad { color: var(--bad); }
  .src { grid-area: src; color: var(--faint); font-size: 10.5px; margin-top: -2px; }
  .row.unmeasured .v { color: var(--faint); font-weight: 400; }
  .bar-row { display: grid; grid-template-columns: 190px 1fr 120px; gap: 10px; align-items: center; padding: 3px 0; }
  .bk { color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar { background: #141312; border-radius: 3px; height: 9px; overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--accent); }
  .bv { text-align: right; font-variant-numeric: tabular-nums; }
  .bv em { color: var(--faint); font-style: normal; }
  .chip { display: inline-flex; gap: 7px; align-items: baseline; padding: 4px 9px; margin: 0 6px 6px 0;
          border: 1px solid var(--line); border-radius: 5px; background: #1c1b1a; }
  .chip b { font-weight: 600; } .chip span { color: var(--faint); font-size: 11px; }
  .chip.good b { color: var(--good); } .chip.warn b { color: var(--warn); } .chip.bad b { color: var(--bad); }
  ul.anom { list-style: none; margin: 0; padding: 0; }
  ul.anom li { padding: 8px 0; border-bottom: 1px solid rgba(255,255,255,.04); }
  ul.anom li:last-child { border-bottom: 0; }
  ul.anom b { font-weight: 600; }
  .sev { display: inline-block; min-width: 62px; margin-right: 9px; padding: 1px 6px; border-radius: 3px;
         font-size: 10px; text-transform: uppercase; letter-spacing: .07em; text-align: center;
         background: #332f2c; color: var(--dim); vertical-align: 1px; }
  .sev-blocker .sev { background: #4a2b27; color: #e79b8f; }
  .sev-high .sev { background: #4a3a22; color: #e0bb72; }
  .truth { color: var(--dim); margin: 3px 0 0 71px; }
  .repair { color: var(--faint); margin: 3px 0 0 71px; font-size: 11.5px; }
  .repair code { color: var(--accent); }
  .empty { color: var(--faint); }
  .stopped { background: #4a2b27; color: #e79b8f; padding: 1px 6px; border-radius: 3px;
             font-size: 10px; letter-spacing: .07em; margin-left: 6px; }
  footer { margin-top: 22px; color: var(--faint); font-size: 11px; line-height: 1.7; }
  footer code { color: var(--dim); }
</style>
<div class="wrap">
  <h1><span class="dot">◆</span> TRIOS CONTINUOUS LOOP</h1>
  <div class="sub">${esc((L.title && L.title.v) || 'no subject recorded')}</div>

  <div class="banner${stale ? ' stale' : ''}">
    reading taken <b>${esc(r && r.at ? r.at : 'never')}</b>
    ${age ? `— <b>${esc(age)}</b> ago` : ''}
    ${stale ? `— OLDER THAN ONE CYCLE (${staleMin}m). Every number below is that old; none of them is live.` : ''}
    <br>every value carries the command that produced it. A dash means nobody measured it — it is never a zero.
  </div>

  <div class="grid">
    <div class="card">
      <h2>The swarm</h2>
${row('bees running', at(q, qv ? qv.running : null), null, qv && Number(qv.running) === 0 ? 'bad' : 'good')}
${row('dispatches finished', at(q, qv ? qv.finished : null))}
${row('candidates refused', at(q, skipTotal), null, 'warn')}
${row('idle, over the window', r ? r.idle : null, (v) => `${v}%`, 'warn')}
    </div>

    <div class="card">
      <h2>This checkout</h2>
${row('branch', at(g, gv ? gv.branch : null))}
${row('commits behind its remote', at(g, gv ? gv.behind : null), null, gv && Number(gv.behind) > 50 ? 'bad' : '')}
${row('commits ahead', at(g, gv ? gv.ahead : null))}
${row('volume used', at(d, dv ? dv.percentUsed : null), (v) => `${v}%`, dv && Number(dv.percentUsed) >= 90 ? 'bad' : dv && Number(dv.percentUsed) >= 80 ? 'warn' : 'good')}
    </div>

    <div class="card">
      <h2>The loop itself${stopped ? ' <span class="stopped">stopped</span>' : ''}</h2>
${row('iteration', stopped ? at(L.iteration, `${L.iteration.v} — STOPPED`) : L.iteration, null, stopped ? 'bad' : '')}
${row('hours since one closed', L.staleHours, (v) => `${v}h`, L.staleHours && Number(L.staleHours.v) > 24 ? 'bad' : '')}
${row('lessons recorded', L.lessons)}
${row('lock', at(lock, lockv ? (lockv.held ? `${lockv.holder} (${lockv.ageMin}m)` : 'free') : null))}
    </div>

    <div class="card">
      <h2>Gates</h2>
${row('declared', at(gates, gatev ? gatev.declared : null))}
${row('wired into a workflow', at(gates, gatev ? gatev.wired : null), null, gatev && Number(gatev.wired) < Number(gatev.declared) ? 'warn' : 'good')}
${row('fires the loop: launchd', at(driver, drv ? drv.launchd : null), null, drv && Number(drv.launchd) > 0 ? 'good' : 'bad')}
${row('fires the loop: crontab', at(driver, drv ? drv.crontab : null))}
    </div>

    ${refusalRows ? `<div class="card full">
      <h2>Why candidates were refused — ${skipTotal} in total</h2>
${refusalRows}
    </div>` : ''}

    ${timerRows ? `<div class="card full">
      <h2>Timers — when each last wrote</h2>
      <div>${timerRows}</div>
    </div>` : ''}

    <div class="card full">
      <h2>Anomalies — what the loop asserts, against what was measured${anomHidden > 0 ? ` <span class="empty">(${anomalies.length} of ${sorted.length} shown, worst first)</span>` : ''}</h2>
      ${anomFailed
        ? '<div class="empty">NOT COMPUTED — the detector threw on this reading. This is not "no anomalies"; nobody looked.</div>'
        : anomalies.length
        ? `<ul class="anom">${anomalies.map((a) => {
            const sev = String(a.severity || '').toLowerCase()
            return `<li class="sev-${esc(sev)}">
          <span class="sev">${esc(a.severity || '?')}</span>
          <b>${esc(a.claim || a.id || 'unnamed')}</b>
          <div class="truth">${esc(a.truth || '')}</div>
          ${a.repair ? `<div class="repair">repair: <code>${esc(a.repair)}</code></div>` : ''}
        </li>`
          }).join('')}</ul>`
        : '<div class="empty">this reading produced none — which is not the same as none present.</div>'}
    </div>
  </div>

  <footer>
    Rendered by <code>tri dash-cc</code> from the newest line of
    <code>state/cycle-readings.jsonl</code>. Nothing on this page is measured here and
    nothing is derived from anything else on it, except the refusal shares, which come
    from two numbers in the same reading.<br>
    A dash is not a zero. A stale page says so at the top, before any number.
  </footer>
</div>
`
}

const isMain = process.argv[1] && process.argv[1].endsWith('/dash-cc.mjs')
if (isMain) {
  const argv = process.argv.slice(2)
  const staleMin = Number(argOf(argv, '--stale-min', 30))
  const r = newestReading()
  if (!r) {
    console.log('no reading in state/cycle-readings.jsonl - run `tri cycle` first')
    console.log('  (nothing was rendered: a dashboard invented from no reading is worse than none)')
    process.exit(1)
  }
  // THE ANOMALIES ARE RE-DERIVED, NOT READ BACK.
  //
  // My first draft read `state/cycle-anomalies.jsonl`. No such file exists - the
  // cycle persists only the COUNT. Had the render silently caught the ENOENT and
  // drawn an empty list, this page would have shown "none" for a loop with two
  // blockers standing, which is the failure mode this whole file is written
  // against. So it calls the cycle's own `findAnomalies` on the same reading.
  // That is not a second measurement: same function, same input, same answer.
  //
  // The catch prints. An anomaly list that cannot be computed is not an empty
  // anomaly list, and the page must not be able to confuse the two.
  let anomalies = []
  try {
    const { findAnomalies } = await import('./anomaly.mjs')
    anomalies = findAnomalies(r) || []
  } catch (e) {
    console.log(`  anomalies could not be computed from this reading: ${e.message}`)
    console.log('  the page will say so rather than show an empty list')
    anomalies = null
  }

  const html = render(r, { anomalies, staleMin })
  if (argv.includes('--stdout')) {
    process.stdout.write(html)
  } else {
    fs.writeFileSync(OUT, html)
    console.log(`wrote ${path.relative(process.cwd(), OUT)}  (${(html.length / 1024).toFixed(1)} kB)`)
    console.log(`  from a reading taken ${r.at}`)
  }

  const ageMs = r.at ? Date.now() - Date.parse(r.at) : null
  if (ageMs === null || ageMs > staleMin * 60000) {
    console.log(`  the reading is ${humanAge(ageMs) || 'of unknown age'} old, over the ${staleMin}m cycle - the page says so at the top`)
    process.exit(2)
  }
}
