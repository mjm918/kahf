/**
 * Account settings page.
 *
 * Displays the user's profile (avatar, first name, last name, email)
 * with editable fields. Provides save functionality for profile
 * updates and a leave workspace action.
 *
 * AccountSettings — standalone component for user profile management.
 */

import { Component, inject, signal, computed, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule, ToastModule } from '@syncfusion/ej2-angular-notifications';
import { AuthService } from '../../core/services/auth.service';
import { UserService } from '../../core/services/user.service';

@Component({
  selector: 'app-account-settings',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, MessageModule, ToastModule],
  templateUrl: './account.html',
})
export class AccountSettings implements OnInit {
  private readonly auth = inject(AuthService);
  private readonly userService = inject(UserService);

  firstName = '';
  lastName = '';
  email = computed(() => this.auth.currentUser()?.email ?? '');
  loading = signal(false);
  error = signal('');
  success = signal('');

  userInitials = computed(() => {
    const user = this.auth.currentUser();
    if (!user) return '?';
    return `${user.first_name?.[0] ?? ''}${user.last_name?.[0] ?? ''}`.toUpperCase();
  });

  ngOnInit(): void {
    const user = this.auth.currentUser();
    if (user) {
      this.firstName = user.first_name;
      this.lastName = user.last_name;
    }
  }

  async onSave(): Promise<void> {
    if (!this.firstName.trim()) {
      this.error.set('First name is required.');
      return;
    }
    this.error.set('');
    this.success.set('');
    this.loading.set(true);
    try {
      await this.userService.updateProfile({
        first_name: this.firstName.trim(),
        last_name: this.lastName.trim(),
      });
      this.success.set('Profile updated successfully.');
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to update profile.');
    } finally {
      this.loading.set(false);
    }
  }
}
