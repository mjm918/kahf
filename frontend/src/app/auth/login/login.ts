/**
 * Login page component.
 *
 * Displays an email + password form using Syncfusion TextBox and Button.
 * On submit, calls AuthService.login and navigates to the dashboard on
 * success. Shows an error message on failure. Links to the signup page.
 */

import { Component, signal } from '@angular/core';
import { Router, RouterLink } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, RouterLink, AuthLayout],
  templateUrl: './login.html',
})
export class Login {
  email = '';
  password = '';
  error = signal('');
  loading = signal(false);

  constructor(private auth: AuthService, private router: Router) {}

  async onSubmit(): Promise<void> {
    this.error.set('');
    this.loading.set(true);
    try {
      await this.auth.login(this.email, this.password);
      this.router.navigate(['/']);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Login failed. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }
}
