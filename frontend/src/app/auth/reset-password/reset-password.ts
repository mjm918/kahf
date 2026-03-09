/**
 * Reset password page component.
 *
 * Displays a form with email (pre-filled from router state), OTP code,
 * new password, and confirm password fields. Password field includes a
 * real-time strength meter using Syncfusion ProgressBar (matching the
 * signup page). Validates password complexity and confirmation match
 * before submission. On submit, calls AuthService.resetPassword.
 * On success, shows a success message and navigates to login.
 */

import { Component, signal, inject, ViewChild, AfterViewInit } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule, TextBoxComponent, OtpInputModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';
import { PasswordStrengthMeter } from '../../shared/components/password-strength/password-strength';

@Component({
  selector: 'app-reset-password',
  standalone: true,
  imports: [FormsModule, TextBoxModule, OtpInputModule, ButtonModule, MessageModule, RouterLink, AuthLayout, PasswordStrengthMeter],
  templateUrl: './reset-password.html',
})
export class ResetPassword implements AfterViewInit {
  @ViewChild('passwordBox') passwordBox!: TextBoxComponent;
  @ViewChild('confirmBox') confirmBox!: TextBoxComponent;
  email = '';
  code = '';
  newPassword = '';
  confirmPassword = '';
  error = signal('');
  success = signal('');
  loading = signal(false);
  passwordStrength = signal(0);
  confirmMismatch = signal(false);

  private auth = inject(AuthService);
  private router = inject(Router);
  private iconsAdded = false;

  constructor() {
    const nav = this.router.getCurrentNavigation();
    this.email = nav?.extras?.state?.['email'] || '';
  }

  ngAfterViewInit(): void {
    this.tryAddPasswordIcons();
  }

  private tryAddPasswordIcons(): void {
    if (this.iconsAdded) return;
    if (this.passwordBox && this.confirmBox) {
      this.addPasswordToggle(this.passwordBox);
      this.addPasswordToggle(this.confirmBox);
      this.iconsAdded = true;
    }
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

  onOtpChange(event: any): void {
    this.code = event.value || '';
  }

  onPasswordStrengthChange(strength: number): void {
    this.passwordStrength.set(strength);
  }

  onConfirmInput(): void {
    this.confirmMismatch.set(
      this.confirmPassword.length > 0 && this.confirmPassword !== this.newPassword
    );
  }

  async onSubmit(): Promise<void> {
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
      const resp = await this.auth.resetPassword(this.email, this.code, this.newPassword);
      this.success.set(resp.message);
      setTimeout(() => this.router.navigate(['/auth/login']), 2000);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Invalid or expired reset code.');
    } finally {
      this.loading.set(false);
    }
  }
}
