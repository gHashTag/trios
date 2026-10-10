// Scratch harness for lane A's rehearsal (not committed): the Queen's /hosting
// route on the memory store, served over HTTP on 127.0.0.1, flag on.
// Env: PORT, TRIOS_HOSTING_ALLOWLIST. Every request is counted in /stats.
import { Hono } from 'hono'
import { createHostingRoute } from './src/api/routes/hosting'
import {
  createHostingQueen,
  parseAllowlist,
} from './src/api/services/hosting-queen'
import { createMemoryHostingStore } from './src/api/services/hosting-store'

const queen = createHostingQueen({
  store: createMemoryHostingStore(),
  now: Date.now,
  allowlist: parseAllowlist(process.env.TRIOS_HOSTING_ALLOWLIST),
})
const counts: Record<string, number> = {}
let cpu = 0
const app = new Hono()
  .use(async (c, next) => {
    const k = `${c.req.method} ${c.req.path.replace(/hosts\/[0-9a-f]+\/beat/, 'hosts/:id/beat').replace(/jobs\/j_\w+/, 'jobs/:id')}`
    counts[k] = (counts[k] ?? 0) + 1
    const before = process.cpuUsage()
    await next()
    const used = process.cpuUsage(before)
    cpu += used.user + used.system
  })
  .get('/stats', (c) => c.json({ counts, queen_cpu_us: cpu }))
  .route(
    '/hosting',
    createHostingRoute({ enabled: () => true, queen: () => queen }),
  )
const server = Bun.serve({
  port: Number(process.env.PORT ?? 8799),
  hostname: '127.0.0.1',
  fetch: app.fetch,
})
console.log(`local queen on ${server.url}`)
