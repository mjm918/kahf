/**
 * Signup page component.
 *
 * Displays name, email, and password inputs. On submit, calls
 * AuthService.signup and navigates to the OTP verification page
 * with the email passed as a query parameter.
 */

import { Component, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

@Component({
  selector: 'app-signup',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, RouterLink, AuthLayout],
  templateUrl: './signup.html',
})
export class Signup {
  name = '';
  email = '';
  password = '';
  error = signal('');
  loading = signal(false);

  constructor(private auth: AuthService, private router: Router) {}

  async onSubmit(): Promise<void> {
    this.error.set('');
    this.loading.set(true);
    try {
      await this.auth.signup(this.email, this.password, this.name);
      this.router.navigate(['/auth/verify-otp'], { queryParams: { email: this.email } });
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Signup failed. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }
}
