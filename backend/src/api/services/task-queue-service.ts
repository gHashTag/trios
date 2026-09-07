/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * TaskQueueService — Manage agent task queue with priorities and retry logic
 */

import { Pool } from 'pg'
import * as crypto from 'node:crypto'
import { logger } from '../../lib/logger'

export interface TaskQueueDeps {
  databaseUrl: string
}

export type TaskStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'

export interface TaskPayload {
  type: string
  data: Record<string, any>
}

export interface CreateTaskInput {
  agentId: string
  taskType: string
  payload: TaskPayload
  priority?: number
  maxRetries?: number
  assignedBy?: string
  metadata?: Record<string, any>
}

export interface TaskItem {
  id: string
  agentId: string
  taskType: string
  payload: TaskPayload
  priority: number
  status: TaskStatus
  retryCount: number
  maxRetries: number
  createdAt: string
  startedAt?: string
  completedAt?: string
  errorMessage?: string
  result?: any
  assignedBy?: string
  metadata?: Record<string, any>
}

export class TaskQueueService {
  private pool: Pool

  constructor(private deps: TaskQueueDeps) {
    this.pool = new Pool({
      connectionString: deps.databaseUrl,
      ssl: deps.databaseUrl.includes('neon.tech')
        ? { rejectUnauthorized: false }
        : undefined,
    })
  }

  async shutdown(): Promise<void> {
    await this.pool.end()
  }

  /**
   * Create a new task in the queue
   */
  async createTask(input: CreateTaskInput): Promise<TaskItem> {
    logger.info('Creating task', {
      agentId: input.agentId,
      taskType: input.taskType,
      priority: input.priority ?? 0,
    })

    try {
      const taskId = crypto.randomUUID()
      const now = new Date().toISOString()

      const result = await this.pool.query(
        `
        INSERT INTO agent_tasks (
          id, agent_id, task_type, payload, priority, status, 
          max_retries, assigned_by, metadata, created_at
        )
        VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8, $9)
        RETURNING *
      `,
        [
          taskId,
          input.agentId,
          input.taskType,
          JSON.stringify(input.payload),
          input.priority ?? 0,
          input.maxRetries ?? 3,
          input.assignedBy ?? null,
          JSON.stringify(input.metadata ?? {}),
          now,
        ],
      )

      return this.mapRowToTask(result.rows[0])
    } catch (error) {
      logger.error('Failed to create task', {
        agentId: input.agentId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to create task: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Get next pending task for an agent (highest priority, oldest first)
   */
  async dequeueNextTask(agentId: string): Promise<TaskItem | null> {
    logger.info('Dequeuing next task', { agentId })

    try {
      const result = await this.pool.query(
        `
        SELECT * FROM agent_tasks
        WHERE agent_id = $1 AND status = 'pending'
        ORDER BY priority DESC, created_at ASC
        LIMIT 1
        FOR UPDATE SKIP LOCKED
      `,
        [agentId],
      )

      if (result.rows.length === 0) {
        return null
      }

      const task = result.rows[0]

      // Mark as running
      await this.pool.query(
        `UPDATE agent_tasks SET status = 'running', started_at = NOW() WHERE id = $1`,
        [task.id],
      )

      return this.mapRowToTask(task)
    } catch (error) {
      logger.error('Failed to dequeue task', {
        agentId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to dequeue task: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Update task status
   */
  async updateTaskStatus(
    taskId: string,
    status: TaskStatus,
    result?: any,
    errorMessage?: string,
  ): Promise<boolean> {
    logger.info('Updating task status', { taskId, status })

    try {
      const updates: string[] = ['status = $2']
      const values: any[] = [taskId, status]
      let paramIndex = 3

      if (status === 'completed') {
        updates.push(`completed_at = NOW()`)
      }

      if (result !== undefined) {
        updates.push(`result = $${paramIndex}`)
        values.push(JSON.stringify(result))
        paramIndex++
      }

      if (errorMessage !== undefined) {
        updates.push(`error_message = $${paramIndex}`)
        values.push(errorMessage)
        paramIndex++
      }

      const updateQuery = `UPDATE agent_tasks SET ${updates.join(', ')} WHERE id = $1`
      const updateResult = await this.pool.query(updateQuery, values)

      return updateResult.rowCount !== null && updateResult.rowCount > 0
    } catch (error) {
      logger.error('Failed to update task status', {
        taskId,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Increment retry count and reset status to pending
   */
  async retryTask(taskId: string): Promise<boolean> {
    logger.info('Retrying task', { taskId })

    try {
      const result = await this.pool.query(
        `
        UPDATE agent_tasks 
        SET 
          status = 'pending',
          retry_count = retry_count + 1,
          started_at = NULL,
          error_message = NULL
        WHERE id = $1 AND retry_count < max_retries
        RETURNING *
      `,
        [taskId],
      )

      return result.rows.length > 0
    } catch (error) {
      logger.error('Failed to retry task', {
        taskId,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Get tasks by status
   */
  async getTasksByStatus(
    agentId: string,
    status: TaskStatus,
    limit: number = 50,
  ): Promise<TaskItem[]> {
    logger.info('Getting tasks by status', { agentId, status, limit })

    try {
      const result = await this.pool.query(
        `SELECT * FROM agent_tasks WHERE agent_id = $1 AND status = $2 ORDER BY created_at DESC LIMIT $3`,
        [agentId, status, limit],
      )

      return result.rows.map((row) => this.mapRowToTask(row))
    } catch (error) {
      logger.error('Failed to get tasks by status', {
        agentId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to get tasks: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Get task by ID
   */
  async getTask(taskId: string): Promise<TaskItem | null> {
    logger.info('Getting task', { taskId })

    try {
      const result = await this.pool.query(`SELECT * FROM agent_tasks WHERE id = $1`, [taskId])

      if (result.rows.length === 0) {
        return null
      }

      return this.mapRowToTask(result.rows[0])
    } catch (error) {
      logger.error('Failed to get task', {
        taskId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to get task: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Cancel a task
   */
  async cancelTask(taskId: string): Promise<boolean> {
    logger.info('Cancelling task', { taskId })

    try {
      const result = await this.pool.query(
        `UPDATE agent_tasks SET status = 'cancelled', completed_at = NOW() WHERE id = $1 AND status NOT IN ('completed', 'cancelled')`,
        [taskId],
      )

      return result.rowCount !== null && result.rowCount > 0
    } catch (error) {
      logger.error('Failed to cancel task', {
        taskId,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Delete a task
   */
  async deleteTask(taskId: string): Promise<boolean> {
    logger.info('Deleting task', { taskId })

    try {
      const result = await this.pool.query(`DELETE FROM agent_tasks WHERE id = $1`, [taskId])
      return result.rowCount !== null && result.rowCount > 0
    } catch (error) {
      logger.error('Failed to delete task', {
        taskId,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Get task history for an agent
   */
  async getTaskHistory(agentId: string, limit: number = 100): Promise<TaskItem[]> {
    logger.info('Getting task history', { agentId, limit })

    try {
      const result = await this.pool.query(
        `SELECT * FROM agent_tasks WHERE agent_id = $1 ORDER BY created_at DESC LIMIT $2`,
        [agentId, limit],
      )

      return result.rows.map((row) => this.mapRowToTask(row))
    } catch (error) {
      logger.error('Failed to get task history', {
        agentId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to get task history: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Get queue statistics
   */
  async getQueueStats(agentId?: string): Promise<{
    total: number
    pending: number
    running: number
    completed: number
    failed: number
  }> {
    logger.info('Getting queue stats', { agentId })

    try {
      const whereClause = agentId ? 'WHERE agent_id = $1' : ''
      const values = agentId ? [agentId] : []

      const result = await this.pool.query(
        `
        SELECT 
          COUNT(*) as total,
          COUNT(*) FILTER (WHERE status = 'pending') as pending,
          COUNT(*) FILTER (WHERE status = 'running') as running,
          COUNT(*) FILTER (WHERE status = 'completed') as completed,
          COUNT(*) FILTER (WHERE status = 'failed') as failed
        FROM agent_tasks ${whereClause}
      `,
        values,
      )

      const row = result.rows[0]
      return {
        total: parseInt(row.total, 10),
        pending: parseInt(row.pending, 10),
        running: parseInt(row.running, 10),
        completed: parseInt(row.completed, 10),
        failed: parseInt(row.failed, 10),
      }
    } catch (error) {
      logger.error('Failed to get queue stats', {
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to get queue stats: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  private mapRowToTask(row: any): TaskItem {
    return {
      id: row.id,
      agentId: row.agent_id,
      taskType: row.task_type,
      payload: parseJsonb(row.payload),
      priority: row.priority,
      status: row.status as TaskStatus,
      retryCount: row.retry_count,
      maxRetries: row.max_retries,
      createdAt: row.created_at,
      startedAt: row.started_at,
      completedAt: row.completed_at,
      errorMessage: row.error_message,
      result: row.result != null ? parseJsonb(row.result) : undefined,
      assignedBy: row.assigned_by,
      metadata: row.metadata != null ? parseJsonb(row.metadata) : undefined,
    }
  }
}

function parseJsonb(value: unknown): any {
  if (value == null) return undefined
  if (typeof value === 'object') return value
  if (typeof value === 'string') {
    try {
      return JSON.parse(value)
    } catch {
      return value
    }
  }
  return value
}
