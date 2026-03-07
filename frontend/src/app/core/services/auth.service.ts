/**
 * Authentication service handling signup, OTP verification, login,
 * token refresh, logout, forgot password, reset password, and
 * registration status checks.
 *
 * Stores JWT tokens in localStorage (remember me) or sessionStorage
 * (session-only). Exposes reactive `isAuthenticated` and `currentUser`
 * signals for use in components and guards. `registrationOpen` checks
 * whether the system allows open registration (only when no users
 * exist yet). Signup accepts first_name, last_name, optional
 * company_name (owner registration), and optional invite_token. All
 * methods delegate to the backend API via the shared Axios instance.
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
  private readonly user = signal<AuthUser | null>(this.loadUser());

  readonly currentUser = this.user.asReadonly();
  readonly isAuthenticated = computed(() => this.user() !== null);

  constructor(private router: Router) {}

  async registrationOpen(): Promise<boolean> {
    const { data } = await api.get<{ open: boolean }>('/auth/registration-status');
    return data.open;
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

  async refreshToken(): Promise<string> {
    const refreshToken = this.getStorage().getItem('refresh_token');
    if (!refreshToken) throw new Error('no refresh token');
    const { data } = await api.post<{ access_token: string }>('/auth/refresh', { refresh_token: refreshToken });
    this.getStorage().setItem('access_token', data.access_token);
    return data.access_token;
  }

  logout(): void {
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    localStorage.removeItem('user');
    localStorage.removeItem('remember_me');
    sessionStorage.removeItem('access_token');
    sessionStorage.removeItem('refresh_token');
    sessionStorage.removeItem('user');
    this.user.set(null);
    this.router.navigate(['/auth/login']);
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
    const u: AuthUser = { user_id: data.user_id, email: data.email, first_name: data.first_name, last_name: data.last_name };
    storage.setItem('user', JSON.stringify(u));
    this.user.set(u);
  }

  private loadUser(): AuthUser | null {
    const raw = this.getStorage().getItem('user');
    if (!raw) return null;
    try { return JSON.parse(raw); } catch { return null; }
  }
}
