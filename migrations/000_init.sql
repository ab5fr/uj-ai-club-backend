-- Final schema (consolidated from historical migrations 000–008).
-- Fresh databases only — do not apply against DBs that already ran the old chain.

CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    full_name VARCHAR(255) NOT NULL DEFAULT '',
    phone_num VARCHAR(50),
    image VARCHAR(512),
    points INTEGER NOT NULL DEFAULT 0,
    rank INTEGER NOT NULL DEFAULT 0,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    university VARCHAR(255),
    major VARCHAR(255),
    university_major_set BOOLEAN NOT NULL DEFAULT FALSE,
    jupyterhub_username VARCHAR(255),
    firebase_uid VARCHAR(128) NOT NULL UNIQUE,
    CONSTRAINT users_role_check CHECK (role IN ('user', 'admin'))
);

CREATE TABLE IF NOT EXISTS leaderboards (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS leaderboard_entries (
    id SERIAL PRIMARY KEY,
    leaderboard_id INTEGER NOT NULL REFERENCES leaderboards(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    points INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS resources (
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

CREATE TABLE IF NOT EXISTS challenges (
    id SERIAL PRIMARY KEY,
    week INTEGER NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    allowed_submissions INTEGER NOT NULL DEFAULT 3,
    is_current BOOLEAN NOT NULL DEFAULT false,
    visible BOOLEAN NOT NULL DEFAULT true,
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT challenges_allowed_submissions_check CHECK (allowed_submissions > 0)
);

CREATE TABLE IF NOT EXISTS challenge_leaderboard (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    points INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

CREATE TABLE IF NOT EXISTS user_stats (
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

CREATE TABLE IF NOT EXISTS contact_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    sender_ip VARCHAR(45),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS quotes (
    id SERIAL PRIMARY KEY,
    text TEXT NOT NULL,
    author TEXT NOT NULL,
    visible BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS certificates (
    id SERIAL PRIMARY KEY,
    level VARCHAR(100) NOT NULL,
    title VARCHAR(255) NOT NULL,
    course_title VARCHAR(255) NOT NULL DEFAULT '',
    cover_image VARCHAR(512),
    first_name VARCHAR(255) NOT NULL,
    second_name VARCHAR(255) NOT NULL,
    coursera_url VARCHAR(1024),
    youtube_url VARCHAR(1024),
    visible BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS articles (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL UNIQUE,
    excerpt TEXT,
    body TEXT NOT NULL,
    cover_image TEXT,
    visible BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS challenge_notebooks (
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
    auto_grade_enabled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_challenge_notebook UNIQUE(challenge_id)
);

CREATE TABLE IF NOT EXISTS challenge_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    notebook_id INTEGER NOT NULL REFERENCES challenge_notebooks(id) ON DELETE CASCADE,
    attempt_number INTEGER NOT NULL DEFAULT 1,
    status VARCHAR(50) NOT NULL DEFAULT 'not_started',
    score FLOAT,
    max_score FLOAT,
    points_awarded INTEGER NOT NULL DEFAULT 0,
    points_credited BOOLEAN NOT NULL DEFAULT false,
    nbgrader_submission_id VARCHAR(255),
    started_at TIMESTAMPTZ,
    submitted_at TIMESTAMPTZ,
    graded_at TIMESTAMPTZ,
    manual_graded_by UUID REFERENCES users(id) ON DELETE SET NULL,
    manual_graded_at TIMESTAMPTZ,
    session_jti VARCHAR(64),
    session_expires_at TIMESTAMPTZ,
    session_revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT submission_status_check CHECK (
        status IN ('not_started', 'in_progress', 'grading_pending', 'graded', 'error')
    )
);

CREATE TABLE IF NOT EXISTS challenge_attempt_overrides (
    id SERIAL PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    extra_attempts INTEGER NOT NULL DEFAULT 0 CHECK (extra_attempts >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_challenge_attempt_override UNIQUE (user_id, challenge_id)
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
CREATE INDEX idx_users_firebase_uid ON users(firebase_uid);
CREATE UNIQUE INDEX idx_users_jupyterhub_username
ON users(jupyterhub_username) WHERE jupyterhub_username IS NOT NULL;

CREATE INDEX idx_certificates_visible ON certificates(visible);
CREATE INDEX idx_articles_visible ON articles(visible);
CREATE INDEX idx_articles_created_at ON articles(created_at DESC);

CREATE INDEX idx_contact_messages_email_created_at
ON contact_messages (LOWER(email), created_at DESC);
CREATE INDEX idx_contact_messages_ip_created_at
ON contact_messages (sender_ip, created_at DESC)
WHERE sender_ip IS NOT NULL;

CREATE INDEX idx_challenge_notebooks_challenge_id ON challenge_notebooks(challenge_id);
CREATE INDEX idx_challenge_notebooks_assignment_name ON challenge_notebooks(assignment_name);
CREATE INDEX idx_challenge_submissions_user_id ON challenge_submissions(user_id);
CREATE INDEX idx_challenge_submissions_challenge_id ON challenge_submissions(challenge_id);
CREATE INDEX idx_challenge_submissions_status ON challenge_submissions(status);
CREATE INDEX idx_challenge_submissions_user_challenge ON challenge_submissions(user_id, challenge_id);
CREATE UNIQUE INDEX idx_challenge_submissions_user_challenge_attempt
ON challenge_submissions(user_id, challenge_id, attempt_number);
CREATE INDEX idx_challenge_submissions_user_challenge_created
ON challenge_submissions(user_id, challenge_id, created_at DESC);
CREATE INDEX idx_challenge_submissions_session_jti
ON challenge_submissions (session_jti)
WHERE session_jti IS NOT NULL;

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
WITH ranked_attempts AS (
    SELECT
        cs.*,
        ROW_NUMBER() OVER (
            PARTITION BY cs.challenge_id, cs.user_id
            ORDER BY cs.points_awarded DESC, cs.graded_at DESC NULLS LAST, cs.created_at DESC
        ) AS rn
    FROM challenge_submissions cs
    WHERE cs.status = 'graded' AND cs.points_awarded > 0
)
SELECT
    ra.challenge_id,
    u.id as user_id,
    u.full_name,
    u.image,
    ra.points_awarded,
    ra.score,
    ra.max_score,
    ra.status,
    ra.graded_at,
    RANK() OVER (PARTITION BY ra.challenge_id ORDER BY ra.points_awarded DESC) as challenge_rank
FROM ranked_attempts ra
JOIN users u ON ra.user_id = u.id
WHERE ra.rn = 1
ORDER BY ra.challenge_id, ra.points_awarded DESC;
