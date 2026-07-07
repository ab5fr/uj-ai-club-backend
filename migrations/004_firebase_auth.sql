-- Firebase Auth migration: reset users and replace password/google auth with firebase_uid
TRUNCATE users CASCADE;

DROP INDEX IF EXISTS idx_users_google_id;

ALTER TABLE users DROP COLUMN IF EXISTS password_hash;
ALTER TABLE users DROP COLUMN IF EXISTS google_id;
ALTER TABLE users ADD COLUMN firebase_uid VARCHAR(128) UNIQUE NOT NULL;

CREATE INDEX idx_users_firebase_uid ON users(firebase_uid);
