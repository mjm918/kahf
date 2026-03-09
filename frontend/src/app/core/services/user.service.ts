/**
 * User profile management service.
 *
 * Handles authenticated user profile updates, password changes, and
 * account deletion. Works with the AuthService to keep the current
 * user state in sync after profile changes.
 *
 * UserService — injectable service for user profile operations.
 */

import { Injectable, inject } from '@angular/core';
import { api } from './api.service';
import { AuthService } from './auth.service';

@Injectable({ providedIn: 'root' })
export class UserService {
  private readonly auth = inject(AuthService);

  async updateProfile(body: { first_name?: string; last_name?: string; avatar_url?: string }): Promise<void> {
    const { data } = await api.patch('/users/me', body);
    this.auth.updateCurrentUser(data);
  }

  async changePassword(currentPassword: string, newPassword: string): Promise<void> {
    await api.patch('/users/me/password', {
      current_password: currentPassword,
      new_password: newPassword,
    });
  }

  async deleteAccount(password: string): Promise<void> {
    await api.delete('/users/me', { data: { password } });
  }
}
