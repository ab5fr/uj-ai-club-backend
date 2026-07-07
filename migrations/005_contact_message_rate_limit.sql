ALTER TABLE contact_messages
ADD COLUMN IF NOT EXISTS sender_ip VARCHAR(45);

CREATE INDEX IF NOT EXISTS idx_contact_messages_email_created_at
ON contact_messages (LOWER(email), created_at DESC);

CREATE INDEX IF NOT EXISTS idx_contact_messages_ip_created_at
ON contact_messages (sender_ip, created_at DESC)
WHERE sender_ip IS NOT NULL;
