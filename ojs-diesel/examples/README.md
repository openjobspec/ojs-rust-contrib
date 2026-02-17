# ojs-diesel Example

A complete application demonstrating transactional job enqueue with Diesel and OJS.

## Prerequisites

- Rust 1.75+
- Docker and Docker Compose

## Running

1. Start the OJS backend and PostgreSQL:
   ```bash
   docker compose up -d
   ```

2. Create the outbox table:
   ```bash
   docker compose exec postgres psql -U postgres -d myapp -c "
   CREATE TABLE ojs_outbox (
       id UUID PRIMARY KEY,
       job_type TEXT NOT NULL,
       args JSONB NOT NULL,
       queue TEXT,
       priority INTEGER NOT NULL DEFAULT 0,
       status TEXT NOT NULL DEFAULT 'pending',
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       published_at TIMESTAMPTZ,
       error_message TEXT,
       retry_count INTEGER NOT NULL DEFAULT 0,
       last_error_at TIMESTAMPTZ
   );
   CREATE INDEX idx_ojs_outbox_pending ON ojs_outbox (status, created_at) WHERE status = 'pending';
   "
   ```

3. Run the application:
   ```bash
   cargo run
   ```

4. Stop the backend:
   ```bash
   docker compose down
   ```
