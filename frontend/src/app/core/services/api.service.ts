/**
 * Centralized HTTP client wrapping Axios.
 *
 * Creates a pre-configured Axios instance pointing at the backend API.
 * Automatically attaches the JWT access token from localStorage to every
 * request via an interceptor. Exports the singleton `api` instance for
 * use across all services.
 */

import axios from 'axios';
import { environment } from '../../../environments/environment';

export const api = axios.create({
  baseURL: environment.apiUrl,
  headers: { 'Content-Type': 'application/json' },
});

api.interceptors.request.use((config) => {
  const token = localStorage.getItem('access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});
