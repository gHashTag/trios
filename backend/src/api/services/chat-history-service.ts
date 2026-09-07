/**
 * @license
 * Copyright 2025 BrowserOS
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * ChatHistoryService — Read and search conversation history from PostgreSQL
 */

import { Pool } from 'pg'
import { logger } from '../../lib/logger'

function parseMetadata(value: unknown): Record<string, any> | undefined {
  if (value == null) return undefined
  if (typeof value === 'object') return value as Record<string, any>
  if (typeof value === 'string') {
    try {
      return JSON.parse(value)
    } catch {
      return undefined
    }
  }
  return undefined
}

export interface ChatHistoryDeps {
  databaseUrl: string
}

export interface ConversationSummary {
  id: string
  profileId: string
  lastMessagedAt: string
  preview: string
  messageCount: number
}

export interface ConversationDetail {
  id: string
  profileId: string
  createdAt: string
  lastMessagedAt: string
  messages: Array<{
    id: string
    role: string
    content: string
    timestamp: string
  }>
}

export interface SearchMatch {
  conversationId: string
  messageId: string
  content: string
  timestamp: string
  role: string
  context?: {
    before?: string
    after?: string
  }
}

export interface CreateConversationInput {
  profileId: string
  title?: string
  metadata?: Record<string, any>
}

export interface CreateConversationResult {
  id: string
  profileId: string
  createdAt: string
  lastMessagedAt: string
  title?: string
  metadata?: Record<string, any>
}

export interface AddMessageInput {
  conversationId: string
  role: string
  content: string
  orderIndex?: number
  metadata?: Record<string, any>
}

export interface AddMessageResult {
  id: string
  conversationId: string
  role: string
  content: string
  timestamp: string
  orderIndex: number
}

export class ChatHistoryService {
  private pool: Pool

  constructor(private deps: ChatHistoryDeps) {
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
   * List all conversations for a profile
   */
  async listConversations(
    profileId: string,
    limit: number = 50,
    offset: number = 0,
  ): Promise<{
    conversations: ConversationSummary[]
    totalCount: number
    hasMore: boolean
  }> {
    logger.info('Listing conversations', { profileId, limit, offset })

    try {
      // Get total count
      const countResult = await this.pool.query(
        `SELECT COUNT(*) FROM conversations WHERE "profileId" = $1`,
        [profileId],
      )
      const totalCount = parseInt(countResult.rows[0].count, 10)

      // Get conversations with preview
      const result = await this.pool.query(
        `
        SELECT 
          c."rowId" as id,
          c."profileId",
          c."lastMessagedAt",
          c."createdAt",
          (
            SELECT STRING_AGG(m.message, ' ' ORDER BY m."createdAt" DESC)
            FROM "conversationMessages" m
            WHERE m."conversationId" = c."rowId"
          ) as preview,
          (
            SELECT COUNT(*) FROM "conversationMessages" m
            WHERE m."conversationId" = c."rowId"
          ) as "messageCount"
        FROM conversations c
        WHERE c."profileId" = $1
        ORDER BY c."lastMessagedAt" DESC
        LIMIT $2 OFFSET $3
      `,
        [profileId, limit, offset],
      )

      const conversations: ConversationSummary[] = result.rows.map((row) => ({
        id: row.id,
        profileId: row.profileId,
        lastMessagedAt: row.lastMessagedAt,
        preview: row.preview || 'No messages yet',
        messageCount: parseInt(row.messageCount, 10),
      }))

      return {
        conversations,
        totalCount,
        hasMore: offset + conversations.length < totalCount,
      }
    } catch (error) {
      logger.error('Failed to list conversations', {
        profileId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to list conversations: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Get full transcript of a specific conversation
   */
  async getConversation(
    conversationId: string,
    limit: number = 100,
    offset: number = 0,
  ): Promise<ConversationDetail> {
    logger.info('Getting conversation', { conversationId, limit, offset })

    try {
      // Get conversation metadata
      const convResult = await this.pool.query(
        `
        SELECT "rowId", "profileId", "createdAt", "lastMessagedAt"
        FROM conversations
        WHERE "rowId" = $1
      `,
        [conversationId],
      )

      if (convResult.rows.length === 0) {
        throw new Error(`Conversation ${conversationId} not found`)
      }

      const conv = convResult.rows[0]

      // Get messages
      const msgResult = await this.pool.query(
        `
        SELECT "rowId", message, role, "createdAt"
        FROM "conversationMessages"
        WHERE "conversationId" = $1
        ORDER BY "orderIndex" ASC
        LIMIT $2 OFFSET $3
      `,
        [conversationId, limit, offset],
      )

      const messages = msgResult.rows.map((row) => ({
        id: row.rowId,
        role: row.role,
        content: row.message,
        timestamp: row.createdAt,
      }))

      return {
        id: conv.rowId,
        profileId: conv.profileId,
        createdAt: conv.createdAt,
        lastMessagedAt: conv.lastMessagedAt,
        messages,
      }
    } catch (error) {
      logger.error('Failed to get conversation', {
        conversationId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to get conversation: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Search across all conversations by text query
   */
  async searchConversations(
    query: string,
    profileId: string,
    limit: number = 20,
  ): Promise<{
    results: SearchMatch[]
    totalMatches: number
  }> {
    logger.info('Searching conversations', { query, profileId, limit })

    try {
      // Search messages using ILIKE (case-insensitive)
      const result = await this.pool.query(
        `
        SELECT 
          m."rowId" as "messageId",
          m."conversationId",
          m.message as content,
          m.role,
          m."createdAt" as timestamp,
          c."profileId"
        FROM "conversationMessages" m
        JOIN conversations c ON m."conversationId" = c."rowId"
        WHERE c."profileId" = $1
          AND m.message ILIKE $2
        ORDER BY m."createdAt" DESC
        LIMIT $3
      `,
        [profileId, `%${query}%`, limit],
      )

      const results: SearchMatch[] = await Promise.all(
        result.rows.map(async (row) => {
          // Get context (previous and next messages)
          const contextQuery = await this.pool.query(
            `
            SELECT message, role, "orderIndex"
            FROM "conversationMessages"
            WHERE "conversationId" = $1
              AND "orderIndex" IN (
                (SELECT "orderIndex" FROM "conversationMessages" WHERE "rowId" = $2) - 1,
                (SELECT "orderIndex" FROM "conversationMessages" WHERE "rowId" = $2) + 1
              )
            ORDER BY "orderIndex" ASC
          `,
            [row.conversationId, row.messageId],
          )

          const context = {
            before:
              contextQuery.rows.length > 0
                ? contextQuery.rows[0].message.slice(0, 100)
                : undefined,
            after:
              contextQuery.rows.length > 1
                ? contextQuery.rows[1].message.slice(0, 100)
                : undefined,
          }

          return {
            conversationId: row.conversationId,
            messageId: row.messageId,
            content: row.content,
            timestamp: row.timestamp,
            role: row.role,
            context,
          }
        }),
      )

      return { results, totalMatches: results.length }
    } catch (error) {
      logger.error('Failed to search conversations', {
        query,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to search conversations: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Delete a conversation
   */
  async deleteConversation(conversationId: string): Promise<boolean> {
    logger.info('Deleting conversation', { conversationId })

    try {
      // Delete messages first (cascade should handle this, but being explicit)
      await this.pool.query(
        `DELETE FROM "conversationMessages" WHERE "conversationId" = $1`,
        [conversationId],
      )

      // Delete conversation
      const result = await this.pool.query(
        `DELETE FROM conversations WHERE "rowId" = $1`,
        [conversationId],
      )

      return result.rowCount !== null && result.rowCount > 0
    } catch (error) {
      logger.error('Failed to delete conversation', {
        conversationId,
        error: error instanceof Error ? error.message : String(error),
      })
      return false
    }
  }

  /**
   * Create a new conversation
   */
  async createConversation(
    input: CreateConversationInput,
  ): Promise<CreateConversationResult> {
    logger.info('Creating conversation', { profileId: input.profileId })

    try {
      const conversationId = `conv-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
      const now = new Date().toISOString()

      const result = await this.pool.query(
        `
        INSERT INTO conversations ("rowId", "profileId", "createdAt", "lastMessagedAt", "title", "metadata")
        VALUES ($1, $2, $3, $3, $4, $5)
        RETURNING "rowId", "profileId", "createdAt", "lastMessagedAt", "title", "metadata"
      `,
        [conversationId, input.profileId, now, input.title || null, JSON.stringify(input.metadata || {})],
      )

      const row = result.rows[0]
      return {
        id: row.rowId,
        profileId: row.profileId,
        createdAt: row.createdAt,
        lastMessagedAt: row.lastMessagedAt,
        title: row.title,
        metadata: parseMetadata(row.metadata),
      }
    } catch (error) {
      logger.error('Failed to create conversation', {
        profileId: input.profileId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to create conversation: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }

  /**
   * Add a message to a conversation
   */
  async addMessage(input: AddMessageInput): Promise<AddMessageResult> {
    logger.info('Adding message to conversation', {
      conversationId: input.conversationId,
      role: input.role,
    })

    try {
      const messageId = `msg-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
      const now = new Date().toISOString()

      // Get max orderIndex if not provided
      let orderIndex = input.orderIndex
      if (orderIndex === undefined) {
        const maxResult = await this.pool.query(
          `SELECT COALESCE(MAX("orderIndex"), -1) as "maxIndex" FROM "conversationMessages" WHERE "conversationId" = $1`,
          [input.conversationId],
        )
        orderIndex = maxResult.rows[0].maxIndex + 1
      }

      // Insert message
      const msgResult = await this.pool.query(
        `
        INSERT INTO "conversationMessages" ("rowId", "conversationId", "message", "role", "orderIndex", "createdAt", "metadata")
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING "rowId", "conversationId", "message", "role", "orderIndex", "createdAt"
      `,
        [messageId, input.conversationId, input.content, input.role, orderIndex, now, JSON.stringify(input.metadata || {})],
      )

      // Update conversation lastMessagedAt
      await this.pool.query(
        `UPDATE conversations SET "lastMessagedAt" = $1 WHERE "rowId" = $2`,
        [now, input.conversationId],
      )

      const row = msgResult.rows[0]
      return {
        id: row.rowId,
        conversationId: row.conversationId,
        role: row.role,
        content: row.message,
        timestamp: row.createdAt,
        orderIndex: row.orderIndex,
      }
    } catch (error) {
      logger.error('Failed to add message', {
        conversationId: input.conversationId,
        error: error instanceof Error ? error.message : String(error),
      })
      throw new Error(
        `Failed to add message: ${error instanceof Error ? error.message : 'Unknown error'}`,
      )
    }
  }
}
