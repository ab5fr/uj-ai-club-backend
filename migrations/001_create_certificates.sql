CREATE TABLE IF NOT EXISTS certificates (
    id SERIAL PRIMARY KEY,
    level VARCHAR(100) NOT NULL,
    title VARCHAR(255) NOT NULL,
    cover_image VARCHAR(512),
    first_name VARCHAR(255) NOT NULL,
    second_name VARCHAR(255) NOT NULL,
    coursera_url VARCHAR(1024),
    youtube_url VARCHAR(1024),
    visible BOOLEAN NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_certificates_visible ON certificates(visible);
