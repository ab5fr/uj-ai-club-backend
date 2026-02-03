-- Combined migration file (ordered)

-- 1) Base schema
CREATE TABLE users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(255) NOT NULL DEFAULT '',
    phone_num VARCHAR(50),
    image VARCHAR(512),
    points INTEGER NOT NULL DEFAULT 0,
    rank INTEGER NOT NULL DEFAULT 0, -- useless remove later
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE users
ADD CONSTRAINT users_role_check CHECK (role IN ('user', 'admin'));

CREATE TABLE leaderboards (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE leaderboard_entries (
    id SERIAL PRIMARY KEY,
    leaderboard_id INTEGER NOT NULL REFERENCES leaderboards(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    points INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE resources (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    provider VARCHAR(255) NOT NULL,
    cover_image VARCHAR(512),
    instructor_name VARCHAR(255) NOT NULL,
    instructor_image VARCHAR(512),
    notion_url VARCHAR(512),
    visible BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE challenges (
    id SERIAL PRIMARY KEY,
    week INTEGER NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    challenge_url VARCHAR(512) NOT NULL,
    is_current BOOLEAN NOT NULL DEFAULT false,
    visible BOOLEAN NOT NULL DEFAULT true,
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE challenge_leaderboard (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    points INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

CREATE TABLE user_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    best_subject VARCHAR(255),
    improveable VARCHAR(255),
    quickest_hunter INTEGER NOT NULL DEFAULT 0,
    challenges_taken INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

CREATE TABLE contact_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE quotes (
    id SERIAL PRIMARY KEY,
    text TEXT NOT NULL,
    author TEXT NOT NULL,
    visible BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO quotes (text, author, visible) VALUES
  ('The only limit to our realization of tomorrow is our doubts of today.', 'Franklin D. Roosevelt', TRUE),
  ('Don''t watch the clock; do what it does. Keep going.', 'Sam Levenson', TRUE),
  ('The future belongs to those who believe in the beauty of their dreams.', 'Eleanor Roosevelt', TRUE),
  ('Be yourself; everyone else is already taken.', 'Oscar Wilde', TRUE),
  ('The secret of getting ahead is getting started.', 'Mark Twain', TRUE),
  ('Not all those who wander are lost.', 'J.R.R. Tolkien', FALSE);

CREATE INDEX idx_leaderboard_entries_leaderboard_id ON leaderboard_entries(leaderboard_id);
CREATE INDEX idx_leaderboard_entries_points ON leaderboard_entries(points DESC);
CREATE INDEX idx_challenge_leaderboard_points ON challenge_leaderboard(points DESC);
CREATE INDEX idx_challenges_is_current ON challenges(is_current);
CREATE INDEX idx_users_points ON users(points DESC);
CREATE INDEX idx_users_role ON users(role);

-- 2) Google OAuth migration
ALTER TABLE users ADD COLUMN google_id VARCHAR(255) UNIQUE;
ALTER TABLE users ADD COLUMN university VARCHAR(255);
ALTER TABLE users ADD COLUMN major VARCHAR(255);
ALTER TABLE users ADD COLUMN university_major_set BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
CREATE INDEX idx_users_google_id ON users(google_id);
UPDATE users SET university_major_set = FALSE WHERE university IS NULL OR major IS NULL;

-- 3) JupyterHub/nbgrader integration
CREATE TABLE challenge_notebooks (
    id SERIAL PRIMARY KEY,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    assignment_name VARCHAR(255) NOT NULL UNIQUE,
    notebook_filename VARCHAR(255) NOT NULL,
    notebook_path VARCHAR(512) NOT NULL,
    max_points INTEGER NOT NULL DEFAULT 100,
    cpu_limit FLOAT NOT NULL DEFAULT 0.5,
    memory_limit VARCHAR(20) NOT NULL DEFAULT '512M',
    time_limit_minutes INTEGER NOT NULL DEFAULT 60,
    network_disabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_challenge_notebook UNIQUE(challenge_id)
);

CREATE TABLE challenge_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    notebook_id INTEGER NOT NULL REFERENCES challenge_notebooks(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'not_started',
    score FLOAT,
    max_score FLOAT,
    points_awarded INTEGER NOT NULL DEFAULT 0,
    points_credited BOOLEAN NOT NULL DEFAULT false,
    nbgrader_submission_id VARCHAR(255),
    started_at TIMESTAMPTZ,
    submitted_at TIMESTAMPTZ,
    graded_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_user_challenge_submission UNIQUE(user_id, challenge_id)
);

ALTER TABLE challenge_submissions
ADD CONSTRAINT submission_status_check CHECK (
    status IN ('not_started', 'in_progress', 'submitted', 'grading', 'graded', 'error')
);

ALTER TABLE users
ADD COLUMN IF NOT EXISTS jupyterhub_username VARCHAR(255);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_jupyterhub_username 
ON users(jupyterhub_username) WHERE jupyterhub_username IS NOT NULL;

CREATE INDEX idx_challenge_notebooks_challenge_id ON challenge_notebooks(challenge_id);
CREATE INDEX idx_challenge_notebooks_assignment_name ON challenge_notebooks(assignment_name);
CREATE INDEX idx_challenge_submissions_user_id ON challenge_submissions(user_id);
CREATE INDEX idx_challenge_submissions_challenge_id ON challenge_submissions(challenge_id);
CREATE INDEX idx_challenge_submissions_status ON challenge_submissions(status);
CREATE INDEX idx_challenge_submissions_user_challenge ON challenge_submissions(user_id, challenge_id);

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_challenge_notebooks_updated_at
    BEFORE UPDATE ON challenge_notebooks
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_challenge_submissions_updated_at
    BEFORE UPDATE ON challenge_submissions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE OR REPLACE VIEW challenge_submission_leaderboard AS
SELECT 
    cs.challenge_id,
    u.id as user_id,
    u.full_name,
    u.image,
    cs.points_awarded,
    cs.score,
    cs.max_score,
    cs.status,
    cs.graded_at,
    RANK() OVER (PARTITION BY cs.challenge_id ORDER BY cs.points_awarded DESC) as challenge_rank
FROM challenge_submissions cs
JOIN users u ON cs.user_id = u.id
WHERE cs.status = 'graded' AND cs.points_awarded > 0
ORDER BY cs.challenge_id, cs.points_awarded DESC;

-- 4) Require password for Google users (no schema changes)
-- This section intentionally has no SQL statements.

-- 5) Set admin user
INSERT INTO users (id, email, password_hash, role)
VALUES (
    gen_random_uuid(),
    'a2005balila@gmail.com',
    '$2b$12$rjvnyy3C5AEt4apM53VzEeZnKMUVmW7FP44rb1Xdxmw1ozPUZFM46',
    'admin'
)
ON CONFLICT (email) DO UPDATE
SET password_hash = EXCLUDED.password_hash,
    role = EXCLUDED.role;
