#!/usr/bin/env bun
/**
 * TRIOS Database Migration Runner
 * Usage: bun run scripts/migrate.ts
 */

import { Pool } from 'pg'
import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'

const DATABASE_URL = Bun.env.DATABASE_URL || 'postgresql://localhost:5432/trios'

async function runMigrations() {
  console.log('🔧 Starting TRIOS database migrations...')
  console.log(`📦 Database: ${DATABASE_URL.includes('neon.tech') ? 'Neon' : 'PostgreSQL'}`)

  const pool = new Pool({
    connectionString: DATABASE_URL,
    ssl: DATABASE_URL.includes('neon.tech') ? { rejectUnauthorized: false } : false,
  })

  try {
    // Create migrations table if not exists
    await pool.query(`
      CREATE TABLE IF NOT EXISTS _trios_migrations (
        name VARCHAR(255) PRIMARY KEY,
        applied_at TIMESTAMPTZ DEFAULT NOW()
      )
    `)

    // Get applied migrations
    const applied = await pool.query('SELECT name FROM _trios_migrations ORDER BY name')
    const appliedNames = new Set(applied.rows.map((r) => r.name))

    // Read migration files
    const migrationsDir = join(import.meta.dir, '..', 'migrations')
    const files = await readdir(migrationsDir)
    const migrationFiles = files
      .filter((f) => f.endsWith('.sql'))
      .sort()

    console.log(`📄 Found ${migrationFiles.length} migration files`)

    // Run pending migrations
    for (const file of migrationFiles) {
      if (appliedNames.has(file)) {
        console.log(`⏭️  Skipping: ${file} (already applied)`)
        continue
      }

      console.log(`🔄 Applying: ${file}...`)
      const sql = await readFile(join(migrationsDir, file), 'utf-8')
      
      const client = await pool.connect()
      try {
        await client.query('BEGIN')
        await client.query(sql)
        await client.query('INSERT INTO _trios_migrations (name) VALUES ($1)', [file])
        await client.query('COMMIT')
        console.log(`✅ Applied: ${file}`)
      } catch (error) {
        await client.query('ROLLBACK')
        console.error(`❌ Failed: ${file}`)
        throw error
      } finally {
        client.release()
      }
    }

    console.log('\n✨ All migrations completed successfully!')
  } catch (error) {
    console.error('Migration failed:', error instanceof Error ? error.message : error)
    process.exit(1)
  } finally {
    await pool.end()
  }
}

runMigrations()
