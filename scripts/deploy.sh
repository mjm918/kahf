#!/usr/bin/env bash
#! Production deploy script for KahfLane.
#! Copies .env.production to .env, pulls latest Docker images, and
#! starts all production services including Caddy reverse proxy.
#! Run from the project root: ./scripts/deploy.sh

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
COMPOSE_FILE="$PROJECT_ROOT/docker/docker-compose.prod.yml"
ENV_FILE="$PROJECT_ROOT/.env.production"

if [ ! -f "$ENV_FILE" ]; then
  echo "ERROR: .env.production not found at $ENV_FILE"
  echo "Copy .env.production.example and fill in your secrets."
  exit 1
fi

cp "$ENV_FILE" "$PROJECT_ROOT/.env"

echo "Pulling latest images..."
docker compose -f "$COMPOSE_FILE" pull

echo "Starting production services..."
docker compose -f "$COMPOSE_FILE" up -d

echo "Waiting for services to be healthy..."
docker compose -f "$COMPOSE_FILE" ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"

echo ""
echo "Production deployment complete."
echo "Services: TimescaleDB, Redis, Meilisearch, MinIO, Caddy"
echo "Use 'docker compose -f docker/docker-compose.prod.yml logs -f' to view logs."
