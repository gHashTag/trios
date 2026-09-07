/**
 * Server wrapper for TRIOS Backend
 * This file handles the actual server startup
 */

import startServer from './index.js'

// Start the server
startServer.then((server: any) => {
  // Server is ready, bun will handle the rest
  console.log('\n🎯 Server object ready for bun serve')
}).catch((error: any) => {
  console.error('❌ Failed to start server:', error)
  process.exit(1)
})
