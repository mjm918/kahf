/**
 * Production environment configuration.
 *
 * API_URL is relative so it goes through the same origin (Caddy proxy).
 */

export const environment = {
  production: true,
  apiUrl: '/api',
};
