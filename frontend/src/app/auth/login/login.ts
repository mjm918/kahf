/**
 * Login page component.
 *
 * Displays an email + password form using Syncfusion TextBox and Button.
 * On submit, calls AuthService.login and navigates to the dashboard on
 * success. Shows an error message on failure. Checks registration status
 * on init — if open registration is available, shows a signup link;
 * otherwise hides it (invite-only mode). Displays a "registration closed"
 * info banner when redirected from the signup page.
 */

import { Component, OnInit, signal } from '@angular/core';
import { Router, RouterLink, ActivatedRoute } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule, CheckBoxModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, CheckBoxModule, RouterLink, AuthLayout],
  templateUrl: './login.html',
})
export class Login implements OnInit {
  email = '';
  password = '';
  rememberMe = true;
  error = signal('');
  loading = signal(false);
  showSignupLink = signal(false);
  registrationClosedMessage = signal(false);

  constructor(
    private auth: AuthService,
    private router: Router,
    private route: ActivatedRoute,
  ) {}

  async ngOnInit(): Promise<void> {
    const msg = this.route.snapshot.queryParamMap.get('message');
    if (msg === 'registration-closed') {
      this.registrationClosedMessage.set(true);
    }

    try {
      const open = await this.auth.registrationOpen();
      this.showSignupLink.set(open);
    } catch {
      this.showSignupLink.set(false);
    }
  }

  async onSubmit(): Promise<void> {
    this.error.set('');
    this.loading.set(true);
    try {
      await this.auth.login(this.email, this.password, this.rememberMe);
      this.router.navigate(['/']);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Login failed. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }
}
