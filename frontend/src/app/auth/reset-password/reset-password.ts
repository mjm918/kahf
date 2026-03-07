/**
 * Reset password page component.
 *
 * Displays a form with email (pre-filled from router state), OTP code,
 * and new password fields. On submit, calls AuthService.resetPassword.
 * On success, shows a toast notification and navigates to login.
 */

import { Component, signal, inject, ViewChild, AfterViewInit } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule, TextBoxComponent } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

@Component({
  selector: 'app-reset-password',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, MessageModule, RouterLink, AuthLayout],
  templateUrl: './reset-password.html',
})
export class ResetPassword implements AfterViewInit {
  @ViewChild('passwordBox') passwordBox!: TextBoxComponent;
  email = '';
  code = '';
  newPassword = '';
  error = signal('');
  success = signal('');
  loading = signal(false);

  ngAfterViewInit(): void {
    if (this.passwordBox) {
      this.passwordBox.addIcon('append', 'e-icons e-eye');
      const el = this.passwordBox.element as HTMLInputElement;
      el.parentElement!
        .querySelector('.e-eye')!
        .addEventListener('click', () => {
          el.type = el.type === 'password' ? 'text' : 'password';
        });
    }
  }

  private auth = inject(AuthService);
  private router = inject(Router);

  constructor() {
    const nav = this.router.getCurrentNavigation();
    this.email = nav?.extras?.state?.['email'] || '';
  }

  async onSubmit(): Promise<void> {
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
