#!/usr/bin/env bash
#! Dev script: kills existing backend/frontend instances, then starts both.
#! Backend (cargo run) starts first, followed by frontend (bun run start).
#! Press Ctrl+C to stop both processes.

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
  echo "Stopped."
  exit 0
}

trap cleanup SIGINT SIGTERM

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
