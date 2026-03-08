#!/usr/bin/env bash
#! Dev script: starts Docker services, then runs backend and frontend.
#! Copies .env.development to .env, starts Docker Compose infra,
#! kills existing backend/frontend instances, then starts both.
#! Press Ctrl+C to stop backend and frontend (Docker services keep running).

set -euo pipefail

BACKEND_PORT=3000
FRONTEND_PORT=4200
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

kill_port() {
  local port=$1
  local pids
  pids=$(lsof -ti :"$port" 2>/dev/null || true)
  if [ -n "$pids" ]; then
    echo "Killing processes on port $port: $pids"
    echo "$pids" | xargs kill -9 2>/dev/null || true
    sleep 1
  fi
}

cleanup() {
  echo ""
  echo "Shutting down..."
  kill "$BACKEND_PID" 2>/dev/null || true
  kill "$FRONTEND_PID" 2>/dev/null || true
  wait "$BACKEND_PID" 2>/dev/null || true
  wait "$FRONTEND_PID" 2>/dev/null || true
  echo "Stopped. Docker services still running — use 'docker compose -f docker/docker-compose.yml down' to stop them."
  exit 0
}

trap cleanup SIGINT SIGTERM

cp "$PROJECT_ROOT/.env.development" "$PROJECT_ROOT/.env"

echo "Starting Docker services..."
docker compose -f "$PROJECT_ROOT/docker/docker-compose.yml" up -d

echo "Waiting for services to be healthy..."
sleep 3

kill_port $BACKEND_PORT
kill_port $FRONTEND_PORT

echo "Starting backend..."
cd "$PROJECT_ROOT"
cargo run --bin kahf &
BACKEND_PID=$!

sleep 3

echo "Starting frontend..."
cd "$PROJECT_ROOT/frontend"
bun run start &
FRONTEND_PID=$!

echo "Backend PID: $BACKEND_PID (port $BACKEND_PORT)"
echo "Frontend PID: $FRONTEND_PID (port $FRONTEND_PORT)"
echo "Press Ctrl+C to stop both."

wait
