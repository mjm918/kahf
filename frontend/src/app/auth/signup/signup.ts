/**
 * Signup page component.
 *
 * On init, checks whether open registration is available via
 * AuthService.registrationOpen(). If registration is closed and no
 * invite token is present, redirects to login with a message. The
 * first person to register becomes the tenant owner; all subsequent
 * users must join via invitation link containing an invite_token
 * query parameter. Displays name, email, password, and confirm
 * password inputs. Password field includes a real-time strength meter
 * using Syncfusion ProgressBar. Validates password complexity and
 * confirms passwords match before submission. On submit, calls
 * AuthService.signup and navigates to the OTP verification page.
 */

import { Component, OnInit, signal } from '@angular/core';
import { Router, RouterLink, ActivatedRoute } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { ProgressBarModule } from '@syncfusion/ej2-angular-progressbar';
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
  selector: 'app-signup',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, ProgressBarModule, RouterLink, AuthLayout],
  templateUrl: './signup.html',
})
export class Signup implements OnInit {
  name = '';
  email = '';
  password = '';
  confirmPassword = '';
  error = signal('');
  loading = signal(false);
  initializing = signal(true);
  passwordStrength = signal(0);
  passwordLabel = signal('');
  passwordColor = signal('#D2D0CE');
  checks = PASSWORD_CHECKS;
  checkResults = signal<boolean[]>([false, false, false, false, false]);
  confirmMismatch = signal(false);
  isOwnerRegistration = signal(false);

  private inviteToken: string | null = null;

  constructor(
    private auth: AuthService,
    private router: Router,
    private route: ActivatedRoute,
  ) {}

  async ngOnInit(): Promise<void> {
    this.inviteToken = this.route.snapshot.queryParamMap.get('invite_token');

    try {
      const open = await this.auth.registrationOpen();

      if (open) {
        this.isOwnerRegistration.set(true);
        this.initializing.set(false);
        return;
      }

      if (!this.inviteToken) {
        this.router.navigate(['/auth/login'], {
          queryParams: { message: 'registration-closed' },
        });
        return;
      }

      this.initializing.set(false);
    } catch {
      this.error.set('Unable to check registration status. Please try again.');
      this.initializing.set(false);
    }
  }

  onConfirmInput(): void {
    this.confirmMismatch.set(
      this.confirmPassword.length > 0 && this.confirmPassword !== this.password
    );
  }

  onPasswordInput(): void {
    this.onConfirmInput();
    const pw = this.password;
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

  async onSubmit(): Promise<void> {
    if (this.passwordStrength() < 60) {
      this.error.set('Password is too weak. Please meet at least 3 of the requirements.');
      return;
    }
    if (this.password !== this.confirmPassword) {
      this.error.set('Passwords do not match.');
      return;
    }
    this.error.set('');
    this.loading.set(true);
    try {
      await this.auth.signup(this.email, this.password, this.name, this.inviteToken ?? undefined);
      this.router.navigate(['/auth/verify-otp'], { queryParams: { email: this.email } });
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Signup failed. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }
}
