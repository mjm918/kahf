/**
 * Centralized HTTP client wrapping Axios.
 *
 * Creates a pre-configured Axios instance pointing at the backend API.
 * Automatically attaches the JWT access token from the active storage
 * (localStorage or sessionStorage based on remember-me) to every request
 * via a request interceptor. The response interceptor for 401 handling
 * and automatic token refresh is set up by AuthService to avoid circular
 * dependency. Exports the singleton `api` instance for use across all
 * services.
 */

import axios from 'axios';
import { environment } from '../../../environments/environment';

export const api = axios.create({
  baseURL: environment.apiUrl,
  headers: { 'Content-Type': 'application/json' },
});

api.interceptors.request.use((config) => {
  const storage = localStorage.getItem('remember_me') === '1' ? localStorage : sessionStorage;
  const token = storage.getItem('access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});
