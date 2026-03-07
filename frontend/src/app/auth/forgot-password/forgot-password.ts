/**
 * Forgot password page component.
 *
 * Displays an email input for requesting a password reset OTP.
 * On submit, calls AuthService.forgotPassword and navigates to the
 * reset password page with the email pre-filled via router state.
 * Shows a generic success message regardless of account existence.
 */

import { Component, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

@Component({
  selector: 'app-forgot-password',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, RouterLink, AuthLayout],
  templateUrl: './forgot-password.html',
})
export class ForgotPassword {
  email = '';
  error = signal('');
  success = signal('');
  loading = signal(false);

  constructor(private auth: AuthService, private router: Router) {}

  async onSubmit(): Promise<void> {
    this.error.set('');
    this.success.set('');
    this.loading.set(true);
    try {
      const resp = await this.auth.forgotPassword(this.email);
      this.success.set(resp.message);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Something went wrong. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }

  goToReset(): void {
    this.router.navigate(['/auth/reset-password'], { state: { email: this.email } });
  }
}
