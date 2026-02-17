# ojs-axum Example

A complete Axum application demonstrating OJS integration.

## Prerequisites

- Rust 1.75+
- Docker and Docker Compose

## Running

1. Start the OJS backend:
   ```bash
   docker compose up -d
   ```

2. Run the web server:
   ```bash
   cargo run --bin main
   ```

3. In another terminal, run the worker:
   ```bash
   cargo run --bin worker
   ```

4. Enqueue a job:
   ```bash
   curl -X POST http://localhost:3000/enqueue \
     -H "Content-Type: application/json" \
     -d '{"job_type": "email.send", "to": "user@example.com"}'
   ```

5. Stop the backend:
   ```bash
   docker compose down
   ```
