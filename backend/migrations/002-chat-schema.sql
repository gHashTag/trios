-- TRIOS Backend Migration 002: Chat Schema Updates
-- Adds title and metadata fields to conversations table

-- Add title column if not exists
DO $$ 
BEGIN 
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                 WHERE table_name = 'conversations' AND column_name = 'title') THEN
    ALTER TABLE conversations ADD COLUMN title VARCHAR(500);
  END IF;
END $$;

-- Add metadata column if not exists
DO $$ 
BEGIN 
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                 WHERE table_name = 'conversations' AND column_name = 'metadata') THEN
    ALTER TABLE conversations ADD COLUMN metadata JSONB DEFAULT '{}';
  END IF;
END $$;

-- Add index on title for search
CREATE INDEX IF NOT EXISTS idx_conversations_title ON conversations USING gin(title gin_trgm_ops);

-- Add index on metadata for filtering
CREATE INDEX IF NOT EXISTS idx_conversations_metadata ON conversations USING gin(metadata);

COMMENT ON COLUMN conversations.title IS 'Human-readable chat title';
COMMENT ON COLUMN conversations.metadata IS 'Additional chat metadata (tags, context, etc.)';
