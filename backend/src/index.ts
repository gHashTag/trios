/**
 * @license
 * Copyright 2025 TRIOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * TRIOS Backend — Agent Network Server
 * 
 * Features:
 * - Detailed error handling with retry logic
 * - Connection health monitoring
 * - Graceful shutdown
 * - Comprehensive logging
 */

import { Hono } from 'hono'
import { logger } from 'hono/logger'
import { cors } from 'hono/cors'
import { createTaskQueueRoutes } from './api/routes/tasks.js'
import { createA2aRoutes } from './api/routes/a2a.js'
import { createChatHistoryRoutes } from './api/routes/chat-history.js'
import { A2aRegistryService } from './api/services/a2a/a2a-registry-service.js'

interface Env {
  DATABASE_URL: string
  PORT: number
  NODE_ENV?: string
}

// Configuration with validation
const config = {
  DATABASE_URL: Bun.env.DATABASE_URL || 'postgresql://localhost:5432/trios',
  PORT: parseInt(Bun.env.PORT || '3000', 10),
  NODE_ENV: Bun.env.NODE_ENV || 'development',
  MAX_RETRIES: 3,
  RETRY_DELAY_MS: 1000,
}

// Custom error handler middleware
const errorHandler = async (c: any, next: () => Promise<void>) => {
  try {
    await next()
  } catch (error: any) {
    const errorDetails = {
      timestamp: new Date().toISOString(),
      path: c.req.path,
      method: c.req.method,
      error: error.message || 'Unknown error',
      stack: config.NODE_ENV === 'development' ? error.stack : undefined,
      details: error.cause ? String(error.cause) : undefined,
    }
    
    console.error('❌ [ERROR]', JSON.stringify(errorDetails, null, 2))
    
    // Return detailed error in development, generic in production
    const response = config.NODE_ENV === 'development' 
      ? { 
          success: false, 
          error: errorDetails.error,
          details: errorDetails.details,
          path: errorDetails.path,
          timestamp: errorDetails.timestamp,
        }
      : { 
          success: false, 
          error: 'Internal server error',
          timestamp: errorDetails.timestamp,
        }
    
    return c.json(response, 500)
  }
}

const app = new Hono()

// Middleware
app.use('*', logger())
app.use('*', cors())
app.use('*', errorHandler)

// Health check with database connectivity test
app.get('/health', async (c) => {
  const health = {
    status: 'ok',
    timestamp: new Date().toISOString(),
    version: '1.0.0',
    database: 'unknown',
    uptime: process.uptime(),
  }
  
  try {
    // Test database connection
    const { Pool } = await import('pg')
    const pool = new Pool({ connectionString: config.DATABASE_URL })
    await pool.query('SELECT 1')
    await pool.end()
    
    health.database = config.DATABASE_URL.includes('neon.tech') ? 'Neon (connected)' : 'PostgreSQL (connected)'
  } catch (error: any) {
    health.status = 'degraded'
    health.database = `Connection failed: ${error.message}`
  }
  
  return c.json(health)
})

// Detailed health endpoint
app.get('/health/detailed', async (c) => {
  const details = {
    timestamp: new Date().toISOString(),
    environment: config.NODE_ENV,
    port: config.PORT,
    database_url: config.DATABASE_URL.includes('@') ? config.DATABASE_URL.split('@')[1] : 'invalid',
    services: {
      tasks: 'initialized',
      a2a: 'initialized',
      chats: 'initialized',
    },
    memory: {
      heap_used: Math.round(process.memoryUsage().heapUsed / 1024 / 1024) + ' MB',
      heap_total: Math.round(process.memoryUsage().heapTotal / 1024 / 1024) + ' MB',
      rss: Math.round(process.memoryUsage().rss / 1024 / 1024) + ' MB',
    },
  }
  
  return c.json(details)
})

// Initialize services with retry logic
async function initializeService<T>(
  name: string,
  factory: () => Promise<T>,
  retries = config.MAX_RETRIES
): Promise<T> {
  let lastError: Error | null = null
  
  for (let attempt = 1; attempt <= retries; attempt++) {
    try {
      console.log(`🔧 Initializing ${name} (attempt ${attempt}/${retries})...`)
      const service = await factory()
      console.log(`✅ ${name} initialized successfully`)
      return service
    } catch (error: any) {
      lastError = error
      console.warn(`⚠️  ${name} initialization failed (attempt ${attempt}/${retries}):`, error.message)
      
      if (attempt < retries) {
        const delay = config.RETRY_DELAY_MS * attempt
        console.log(`⏳ Retrying in ${delay}ms...`)
        await new Promise(resolve => setTimeout(resolve, delay))
      }
    }
  }
  
  throw new Error(`Failed to initialize ${name} after ${retries} attempts: ${lastError?.message}`)
}

// Main server startup with error handling
async function startServer() {
  console.log('🚀 TRIOS Backend starting...')
  console.log('📋 Configuration:', {
    port: config.PORT,
    database: config.DATABASE_URL.includes('@') ? config.DATABASE_URL.split('@')[1] : 'invalid',
    environment: config.NODE_ENV,
    max_retries: config.MAX_RETRIES,
  })
  
  try {
    // Initialize services with retry logic
    const a2aService = await initializeService(
      'A2aRegistryService',
      () => Promise.resolve(new A2aRegistryService(config.DATABASE_URL))
    )
    
    // API Routes
    app.route('/api/tasks', createTaskQueueRoutes({ databaseUrl: config.DATABASE_URL }))
    console.log('✅ Tasks API routes registered')
    
    app.route('/api/a2a', createA2aRoutes({ service: a2aService }))
    console.log('✅ A2A API routes registered')
    
    app.route('/api/chats', createChatHistoryRoutes({ databaseUrl: config.DATABASE_URL }))
    console.log('✅ Chat History API routes registered')
    
    // Start server
    const server = {
      port: config.PORT,
      fetch: app.fetch,
    }
    
    console.log('\n✅ TRIOS Backend ready!')
    console.log(`📡 Server listening on http://localhost:${config.PORT}`)
    console.log('\n📋 Available endpoints:')
    console.log('   Health:')
    console.log('     - GET  /health           - Basic health check')
    console.log('     - GET  /health/detailed  - Detailed health with metrics')
    console.log('   Tasks:')
    console.log('     - POST /api/tasks          - Create task')
    console.log('     - GET  /api/tasks/queue/:id - Dequeue task')
    console.log('     - PUT  /api/tasks/:id       - Update task')
    console.log('     - GET  /api/tasks/:id       - Get task by ID')
    console.log('   A2A:')
    console.log('     - POST /api/a2a/message    - Send message to agent')
    console.log('     - POST /api/a2a/registry   - Register agent')
    console.log('     - GET  /api/a2a/matrix     - Get agent matrix')
    console.log('   Chats:')
    console.log('     - GET  /api/chats          - List chats')
    console.log('     - POST /api/chats          - Create chat')
    console.log('     - GET  /api/chats/:id      - Get chat by ID')
    console.log('     - POST /api/chats/:id/messages - Add message')
    console.log('\n🌍 Environment:', config.NODE_ENV)
    if (config.NODE_ENV === 'development') {
      console.log('⚠️  Running in development mode - detailed errors enabled')
    }
    
    return Bun.serve(server)
  } catch (error: any) {
    console.error('\n❌ FATAL: Failed to start TRIOS Backend')
    console.error('Error:', error.message)
    console.error('Stack:', error.stack)
    console.error('\n💡 Troubleshooting:')
    console.error('   1. Check DATABASE_URL is correct')
    console.error('   2. Ensure PostgreSQL is running')
    console.error('   3. Verify migrations are applied: bun run scripts/migrate.ts')
    console.error('   4. Check port is not in use: lsof -i :3000')
    process.exit(1)
  }
}

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('\n🛑 SIGTERM received. Shutting down gracefully...')
  process.exit(0)
})

process.on('SIGINT', () => {
  console.log('\n🛑 SIGINT received. Shutting down gracefully...')
  process.exit(0)
})

// Start server
export default startServer()
