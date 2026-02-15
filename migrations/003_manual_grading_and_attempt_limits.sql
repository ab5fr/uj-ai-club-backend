ALTER TABLE challenges
ADD COLUMN IF NOT EXISTS allowed_submissions INTEGER NOT NULL DEFAULT 3;

ALTER TABLE challenges
ADD CONSTRAINT challenges_allowed_submissions_check CHECK (allowed_submissions > 0);

ALTER TABLE challenge_submissions
ADD COLUMN IF NOT EXISTS attempt_number INTEGER;

ALTER TABLE challenge_submissions
ADD COLUMN IF NOT EXISTS manual_graded_by UUID REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE challenge_submissions
ADD COLUMN IF NOT EXISTS manual_graded_at TIMESTAMPTZ;

UPDATE challenge_submissions
SET attempt_number = 1
WHERE attempt_number IS NULL;

ALTER TABLE challenge_submissions
ALTER COLUMN attempt_number SET NOT NULL;

ALTER TABLE challenge_submissions
ALTER COLUMN attempt_number SET DEFAULT 1;

ALTER TABLE challenge_submissions
DROP CONSTRAINT IF EXISTS unique_user_challenge_submission;

UPDATE challenge_submissions
SET status = 'grading_pending'
WHERE status IN ('submitted', 'grading');

ALTER TABLE challenge_submissions
DROP CONSTRAINT IF EXISTS submission_status_check;

ALTER TABLE challenge_submissions
ADD CONSTRAINT submission_status_check CHECK (
    status IN ('not_started', 'in_progress', 'grading_pending', 'graded', 'error')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_challenge_submissions_user_challenge_attempt
ON challenge_submissions(user_id, challenge_id, attempt_number);

DROP INDEX IF EXISTS idx_challenge_submissions_user_challenge;

CREATE INDEX IF NOT EXISTS idx_challenge_submissions_user_challenge
ON challenge_submissions(user_id, challenge_id);

CREATE INDEX IF NOT EXISTS idx_challenge_submissions_user_challenge_created
ON challenge_submissions(user_id, challenge_id, created_at DESC);

DROP VIEW IF EXISTS challenge_submission_leaderboard;

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