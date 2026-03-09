/**
 * Change password settings page.
 *
 * Provides a form for changing the authenticated user's password.
 * Requires the current password for verification. Includes a
 * reusable password strength meter and confirmation matching.
 * Each password field has an eye icon toggle for show/hide.
 * On success, all sessions are invalidated and the user must
 * log in again.
 *
 * PasswordSettings — standalone component for password change.
 */

import { Component, inject, signal, ViewChild, AfterViewInit } from '@angular/core';
import { Router } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule, TextBoxComponent } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { UserService } from '../../core/services/user.service';
import { PasswordStrengthMeter } from '../../shared/components/password-strength/password-strength';

@Component({
  selector: 'app-password-settings',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, MessageModule, PasswordStrengthMeter],
  templateUrl: './password.html',
})
export class PasswordSettings implements AfterViewInit {
  @ViewChild('currentBox') currentBox!: TextBoxComponent;
  @ViewChild('newBox') newBox!: TextBoxComponent;
  @ViewChild('confirmBox') confirmBox!: TextBoxComponent;

  private readonly userService = inject(UserService);
  private readonly router = inject(Router);

  currentPassword = '';
  newPassword = '';
  confirmPassword = '';
  loading = signal(false);
  error = signal('');
  success = signal('');
  passwordStrength = signal(0);
  confirmMismatch = signal(false);

  ngAfterViewInit(): void {
    this.addPasswordToggle(this.currentBox);
    this.addPasswordToggle(this.newBox);
    this.addPasswordToggle(this.confirmBox);
  }

  private addPasswordToggle(box: TextBoxComponent): void {
    box.addIcon('append', 'e-icons e-eye');
    const el = box.element as HTMLInputElement;
    el.parentElement!
      .querySelector('.e-eye')!
      .addEventListener('click', () => {
        el.type = el.type === 'password' ? 'text' : 'password';
      });
  }

  onPasswordStrengthChange(strength: number): void {
    this.passwordStrength.set(strength);
  }

  onConfirmInput(): void {
    this.confirmMismatch.set(
      this.confirmPassword.length > 0 && this.confirmPassword !== this.newPassword
    );
  }

  async onChangePassword(): Promise<void> {
    if (!this.currentPassword) {
      this.error.set('Current password is required.');
      return;
    }
    if (this.passwordStrength() < 60) {
      this.error.set('Password is too weak. Please meet at least 3 of the requirements.');
      return;
    }
    if (this.newPassword !== this.confirmPassword) {
      this.error.set('Passwords do not match.');
      return;
    }
    this.error.set('');
    this.success.set('');
    this.loading.set(true);
    try {
      await this.userService.changePassword(this.currentPassword, this.newPassword);
      this.success.set('Password changed. You will be redirected to login.');
      this.currentPassword = '';
      this.newPassword = '';
      this.confirmPassword = '';
      setTimeout(() => this.router.navigate(['/auth/login']), 2000);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to change password.');
    } finally {
      this.loading.set(false);
    }
  }
}
