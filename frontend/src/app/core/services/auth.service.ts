/**
 * Authentication service handling signup, OTP verification, login,
 * token refresh, and logout.
 *
 * Stores JWT tokens in localStorage. Exposes reactive `isAuthenticated`
 * and `currentUser` signals for use in components and guards. All
 * methods delegate to the backend API via the shared Axios instance.
 */

import { Injectable, signal, computed } from '@angular/core';
import { Router } from '@angular/router';
import { api } from './api.service';

export interface AuthUser {
  user_id: string;
  email: string;
  name: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  user_id: string;
  email: string;
  name: string;
}

export interface SignupResponse {
  user_id: string;
  email: string;
  message: string;
}

@Injectable({ providedIn: 'root' })
export class AuthService {
  private readonly user = signal<AuthUser | null>(this.loadUser());

  readonly currentUser = this.user.asReadonly();
  readonly isAuthenticated = computed(() => this.user() !== null);

  constructor(private router: Router) {}

  async signup(email: string, password: string, name: string): Promise<SignupResponse> {
    const { data } = await api.post<SignupResponse>('/auth/signup', { email, password, name });
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

  async login(email: string, password: string): Promise<void> {
    const { data } = await api.post<AuthResponse>('/auth/login', { email, password });
    this.storeTokens(data);
  }

  async refreshToken(): Promise<string> {
    const refreshToken = localStorage.getItem('refresh_token');
    if (!refreshToken) throw new Error('no refresh token');
    const { data } = await api.post<{ access_token: string }>('/auth/refresh', { refresh_token: refreshToken });
    localStorage.setItem('access_token', data.access_token);
    return data.access_token;
  }

  logout(): void {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('user');
    this.user.set(null);
    this.router.navigate(['/auth/login']);
  }

  private storeTokens(data: AuthResponse): void {
    localStorage.setItem('access_token', data.access_token);
    localStorage.setItem('refresh_token', data.refresh_token);
    const u: AuthUser = { user_id: data.user_id, email: data.email, name: data.name };
    localStorage.setItem('user', JSON.stringify(u));
    this.user.set(u);
  }

  private loadUser(): AuthUser | null {
    const raw = localStorage.getItem('user');
    if (!raw) return null;
    try { return JSON.parse(raw); } catch { return null; }
  }
}
