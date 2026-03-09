/**
 * Authentication service with server-validated session management.
 *
 * Stores only JWT tokens in localStorage/sessionStorage — user data is
 * decoded from the access token payload (no separate user JSON that can
 * be tampered with). On app bootstrap, validates the stored token against
 * the backend via GET /api/users/me. If invalid, clears all tokens and
 * marks the user as unauthenticated.
 *
 * The `initialized` signal tracks whether the initial validation has
 * completed. Guards must await `ensureInitialized()` before checking
 * `isAuthenticated`.
 *
 * AuthUser — user identity decoded from the JWT access token.
 * AuthResponse — backend response for login/verify-otp/refresh.
 * SignupResponse — backend response for signup (no tokens).
 * AuthService — injectable singleton managing authentication state.
 */

import { Injectable, signal, computed } from '@angular/core';
import { Router } from '@angular/router';
import { api } from './api.service';

export interface AuthUser {
  user_id: string;
  email: string;
  first_name: string;
  last_name: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  user_id: string;
  email: string;
  first_name: string;
  last_name: string;
}

export interface SignupResponse {
  user_id: string;
  email: string;
  message: string;
}

@Injectable({ providedIn: 'root' })
export class AuthService {
  private readonly user = signal<AuthUser | null>(null);
  private readonly _initialized = signal(false);
  private _initPromise: Promise<void> | null = null;

  readonly currentUser = this.user.asReadonly();
  readonly isAuthenticated = computed(() => this.user() !== null);
  readonly initialized = this._initialized.asReadonly();

  constructor(private router: Router) {
    this.setupInterceptors();
  }

  async ensureInitialized(): Promise<void> {
    if (this._initialized()) return;
    if (!this._initPromise) {
      this._initPromise = this.validateSession();
    }
    return this._initPromise;
  }

  async registrationOpen(): Promise<boolean> {
    const { data } = await api.get<{ open: boolean }>('/auth/registration-status');
    return data.open;
  }

  async validateInvite(token: string): Promise<{ email: string; expires_at: string }> {
    const { data } = await api.get<{ email: string; expires_at: string }>(`/auth/invite/validate/${token}`);
    return data;
  }

  async signup(email: string, password: string, firstName: string, lastName: string, companyName?: string, inviteToken?: string): Promise<SignupResponse> {
    const { data } = await api.post<SignupResponse>('/auth/signup', {
      email,
      password,
      first_name: firstName,
      last_name: lastName,
      company_name: companyName,
      invite_token: inviteToken,
    });
    return data;
  }

  async verifyOtp(email: string, code: string): Promise<void> {
    const { data } = await api.post<AuthResponse>('/auth/verify-otp', { email, code });
    this.storeTokens(data);
  }

  async resendOtp(email: string): Promise<SignupResponse> {
    const { data } = await api.post<SignupResponse>('/auth/resend-otp', { email });
    return data;
  }

  async login(email: string, password: string, rememberMe: boolean = true): Promise<void> {
    const { data } = await api.post<AuthResponse>('/auth/login', { email, password });
    this.storeTokens(data, rememberMe);
  }

  async forgotPassword(email: string): Promise<{ message: string }> {
    const { data } = await api.post<{ message: string }>('/auth/forgot-password', { email });
    return data;
  }

  async resetPassword(email: string, code: string, newPassword: string): Promise<{ message: string }> {
    const { data } = await api.post<{ message: string }>('/auth/reset-password', { email, code, new_password: newPassword });
    return data;
  }

  updateCurrentUser(data: Partial<AuthUser>): void {
    const current = this.user();
    if (current) {
      this.user.set({ ...current, ...data });
    }
  }

  async logout(): Promise<void> {
    try {
      await api.post('/auth/logout');
    } catch {
      /* best-effort */
    }
    this.clearTokens();
    this.router.navigate(['/auth/login']);
  }

  private async validateSession(): Promise<void> {
    const storage = this.getStorage();
    const token = storage.getItem('access_token');

    if (!token) {
      this._initialized.set(true);
      return;
    }

    try {
      const { data } = await api.get<AuthUser>('/users/me');
      this.user.set(data);
    } catch {
      this.clearTokens();
    } finally {
      this._initialized.set(true);
    }
  }

  private setupInterceptors(): void {
    let isRefreshing = false;
    let pendingRequests: Array<{ resolve: (token: string) => void; reject: (err: unknown) => void }> = [];

    api.interceptors.response.use(
      (response) => response,
      async (error) => {
        const original = error.config;

        if (error.response?.status !== 401 || original._retry || original.url?.includes('/auth/')) {
          return Promise.reject(error);
        }

        if (isRefreshing) {
          return new Promise((resolve, reject) => {
            pendingRequests.push({
              resolve: (token: string) => {
                original.headers.Authorization = `Bearer ${token}`;
                resolve(api(original));
              },
              reject,
            });
          });
        }

        original._retry = true;
        isRefreshing = true;

        try {
          const newToken = await this.refreshToken();
          pendingRequests.forEach(p => p.resolve(newToken));
          pendingRequests = [];
          original.headers.Authorization = `Bearer ${newToken}`;
          return api(original);
        } catch (refreshError) {
          pendingRequests.forEach(p => p.reject(refreshError));
          pendingRequests = [];
          this.clearTokens();
          this.router.navigate(['/auth/login']);
          return Promise.reject(refreshError);
        } finally {
          isRefreshing = false;
        }
      }
    );
  }

  private async refreshToken(): Promise<string> {
    const storage = this.getStorage();
    const refreshToken = storage.getItem('refresh_token');
    if (!refreshToken) throw new Error('no refresh token');

    const { data } = await api.post<AuthResponse>('/auth/refresh', { refresh_token: refreshToken });

    storage.setItem('access_token', data.access_token);
    storage.setItem('refresh_token', data.refresh_token);
    this.user.set({
      user_id: data.user_id,
      email: data.email,
      first_name: data.first_name,
      last_name: data.last_name,
    });

    return data.access_token;
  }

  private getStorage(): Storage {
    return localStorage.getItem('remember_me') === '1' ? localStorage : sessionStorage;
  }

  private storeTokens(data: AuthResponse, rememberMe: boolean = true): void {
    if (rememberMe) {
      localStorage.setItem('remember_me', '1');
    } else {
      localStorage.removeItem('remember_me');
    }
    const storage = this.getStorage();
    storage.setItem('access_token', data.access_token);
    storage.setItem('refresh_token', data.refresh_token);
    this.user.set({
      user_id: data.user_id,
      email: data.email,
      first_name: data.first_name,
      last_name: data.last_name,
    });
  }

  private clearTokens(): void {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('remember_me');
    localStorage.removeItem('user');
    sessionStorage.removeItem('access_token');
    sessionStorage.removeItem('refresh_token');
    sessionStorage.removeItem('user');
    this.user.set(null);
  }
}
