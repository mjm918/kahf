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
import { ProgressBarModule } from '@syncfusion/ej2-angular-progressbar';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

interface PasswordCheck {
  label: string;
  test: (pw: string) => boolean;
}

const PASSWORD_CHECKS: PasswordCheck[] = [
  { label: '8+ characters', test: (pw) => pw.length >= 8 },
  { label: 'Uppercase letter', test: (pw) => /[A-Z]/.test(pw) },
  { label: 'Lowercase letter', test: (pw) => /[a-z]/.test(pw) },
  { label: 'Number', test: (pw) => /\d/.test(pw) },
  { label: 'Special character', test: (pw) => /[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(pw) },
];

@Component({
  selector: 'app-reset-password',
  standalone: true,
  imports: [FormsModule, TextBoxModule, OtpInputModule, ButtonModule, ProgressBarModule, MessageModule, RouterLink, AuthLayout],
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
  passwordLabel = signal('');
  passwordColor = signal('#D2D0CE');
  checks = PASSWORD_CHECKS;
  checkResults = signal<boolean[]>([false, false, false, false, false]);
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

  onPasswordInput(): void {
    this.onConfirmInput();
    const pw = this.newPassword;
    const results = this.checks.map((c) => c.test(pw));
    this.checkResults.set(results);

    const passed = results.filter(Boolean).length;
    const percent = (passed / this.checks.length) * 100;
    this.passwordStrength.set(percent);

    if (pw.length === 0) {
      this.passwordLabel.set('');
      this.passwordColor.set('#D2D0CE');
    } else if (percent <= 40) {
      this.passwordLabel.set('Weak');
      this.passwordColor.set('#D13438');
    } else if (percent <= 60) {
      this.passwordLabel.set('Fair');
      this.passwordColor.set('#CA5010');
    } else if (percent <= 80) {
      this.passwordLabel.set('Good');
      this.passwordColor.set('#0078D4');
    } else {
      this.passwordLabel.set('Strong');
      this.passwordColor.set('#107C10');
    }
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
