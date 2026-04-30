CREATE SCHEMA IF NOT EXISTS ssot;
CREATE TABLE IF NOT EXISTS ssot.chapters (
  id SERIAL PRIMARY KEY,
  ch_num TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'stub',
  issue_url TEXT,
  canonical_md TEXT,
  tex_pr_url TEXT,
  word_count INT,
  theorems_count INT DEFAULT 0,
  evidence_axis INT,
  phi_seal BOOLEAN DEFAULT false,
  priority TEXT,
  updated_at TIMESTAMPTZ DEFAULT now()
);
CREATE TABLE IF NOT EXISTS ssot.one_shots (
  id SERIAL PRIMARY KEY,
  chapter_id INT REFERENCES ssot.chapters(id) ON DELETE CASCADE,
  directive TEXT NOT NULL,
  preconditions JSONB,
  deliverables JSONB,
  status TEXT DEFAULT 'pending',
  agent_runs INT DEFAULT 0,
  created_at TIMESTAMPTZ DEFAULT now(),
  completed_at TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS ssot.seeds (
  id SERIAL PRIMARY KEY,
  seed_name TEXT UNIQUE NOT NULL,
  seed_type TEXT,
  repo TEXT,
  ref_url TEXT,
  linked_chapters TEXT[],
  linked_theorems TEXT[],
  status TEXT DEFAULT 'alive',
  phi_weight NUMERIC,
  notes TEXT
);
CREATE TABLE IF NOT EXISTS ssot.theorems (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL,
  canonical_file TEXT NOT NULL,
  qed_status TEXT NOT NULL,
  inv_num TEXT,
  chapter_id INT REFERENCES ssot.chapters(id) ON DELETE SET NULL,
  statement TEXT,
  content_hash TEXT,
  UNIQUE(name, canonical_file)
);
CREATE TABLE IF NOT EXISTS ssot.agent_runs (
  id SERIAL PRIMARY KEY,
  one_shot_id INT REFERENCES ssot.one_shots(id) ON DELETE SET NULL,
  started_at TIMESTAMPTZ DEFAULT now(),
  finished_at TIMESTAMPTZ,
  pr_url TEXT,
  ci_green BOOLEAN,
  tokens_used INT,
  agent_id TEXT,
  notes TEXT
);
CREATE INDEX IF NOT EXISTS idx_chapters_ch_num ON ssot.chapters(ch_num);
CREATE INDEX IF NOT EXISTS idx_chapters_status ON ssot.chapters(status);
CREATE INDEX IF NOT EXISTS idx_one_shots_status ON ssot.one_shots(status);
CREATE INDEX IF NOT EXISTS idx_seeds_type ON ssot.seeds(seed_type);
CREATE INDEX IF NOT EXISTS idx_theorems_inv ON ssot.theorems(inv_num);
CREATE INDEX IF NOT EXISTS idx_theorems_status ON ssot.theorems(qed_status);
