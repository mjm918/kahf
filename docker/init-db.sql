--! Initializes the kahflane database with the TimescaleDB extension.
--! Runs automatically on first container start via docker-entrypoint-initdb.d.

CREATE EXTENSION IF NOT EXISTS timescaledb;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
