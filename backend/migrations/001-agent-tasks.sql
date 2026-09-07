-- TRIOS Backend Migration 001: Agent Tasks Table
-- Creates the agent_tasks table for task queue functionality

CREATE TABLE IF NOT EXISTS agent_tasks (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id VARCHAR(255) NOT NULL,
  task_type VARCHAR(255) NOT NULL,
  payload JSONB NOT NULL DEFAULT '{}',
  priority INTEGER NOT NULL DEFAULT 0,
  status VARCHAR(50) NOT NULL DEFAULT 'pending',
  retry_count INTEGER NOT NULL DEFAULT 0,
  max_retries INTEGER NOT NULL DEFAULT 3,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  error_message TEXT,
  result JSONB,
  assigned_by VARCHAR(255),
  metadata JSONB DEFAULT '{}'
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_agent_tasks_agent_id ON agent_tasks(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_status ON agent_tasks(status);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_priority ON agent_tasks(priority DESC, created_at ASC);
CREATE INDEX IF NOT EXISTS idx_agent_tasks_created_at ON agent_tasks(created_at DESC);

-- Composite index for dequeue operations
CREATE INDEX IF NOT EXISTS idx_agent_tasks_dequeue 
  ON agent_tasks(agent_id, status, priority DESC, created_at ASC) 
  WHERE status = 'pending';

COMMENT ON TABLE agent_tasks IS 'Task queue for TRIOS agent network';
COMMENT ON COLUMN agent_tasks.status IS 'pending, running, completed, failed, cancelled';
