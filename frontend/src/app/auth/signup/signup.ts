/**
 * Signup page component.
 *
 * On init, checks whether open registration is available via
 * AuthService.registrationOpen(). If registration is closed and no
 * invite token is present, redirects to login with a message. The
 * first person to register becomes the tenant owner and must provide
 * a company name; all subsequent users must join via invitation link
 * containing an invite_token query parameter. Displays first name,
 * last name, email, password, and confirm password inputs. For owner
 * registration, also shows a company name field under an Organization
 * section. Password field includes a real-time strength meter using
 * Syncfusion ProgressBar. Validates all fields on blur and before
 * submission. On submit, calls AuthService.signup and navigates to
 * the OTP verification page.
 */

import { Component, OnInit, signal, ViewChild, AfterViewInit, ChangeDetectorRef } from '@angular/core';
import { Router, RouterLink, ActivatedRoute } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule, TextBoxComponent } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { ProgressBarModule } from '@syncfusion/ej2-angular-progressbar';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

interface PasswordCheck {
  label: string;
  test: (pw: string) => boolean;
}

interface FieldErrors {
  firstName: string;
  lastName: string;
  email: string;
  companyName: string;
}

const PASSWORD_CHECKS: PasswordCheck[] = [
  { label: '8+ characters', test: (pw) => pw.length >= 8 },
  { label: 'Uppercase letter', test: (pw) => /[A-Z]/.test(pw) },
  { label: 'Lowercase letter', test: (pw) => /[a-z]/.test(pw) },
  { label: 'Number', test: (pw) => /\d/.test(pw) },
  { label: 'Special character', test: (pw) => /[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(pw) },
];

const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

@Component({
  selector: 'app-signup',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, ProgressBarModule, MessageModule, RouterLink, AuthLayout],
  templateUrl: './signup.html',
})
export class Signup implements OnInit, AfterViewInit {
  @ViewChild('passwordBox') passwordBox!: TextBoxComponent;
  @ViewChild('confirmBox') confirmBox!: TextBoxComponent;
  firstName = '';
  lastName = '';
  email = '';
  password = '';
  confirmPassword = '';
  companyName = '';
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
  emailReadonly = signal(false);
  fieldErrors = signal<FieldErrors>({ firstName: '', lastName: '', email: '', companyName: '' });

  private inviteToken: string | null = null;

  private addPasswordToggle(box: TextBoxComponent): void {
    box.addIcon('append', 'e-icons e-eye');
    const el = box.element as HTMLInputElement;
    el.parentElement!
      .querySelector('.e-eye')!
      .addEventListener('click', () => {
        el.type = el.type === 'password' ? 'text' : 'password';
      });
  }

  private iconsAdded = false;

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

  constructor(
    private auth: AuthService,
    private router: Router,
    private route: ActivatedRoute,
    private cdr: ChangeDetectorRef,
  ) {}

  async ngOnInit(): Promise<void> {
    this.inviteToken = this.route.snapshot.queryParamMap.get('invite_token');

    try {
      const open = await this.auth.registrationOpen();

      if (open) {
        this.isOwnerRegistration.set(true);
        this.initializing.set(false);
        this.cdr.detectChanges();
        this.tryAddPasswordIcons();
        return;
      }

      if (!this.inviteToken) {
        this.router.navigate(['/auth/login'], {
          queryParams: { message: 'registration-closed' },
        });
        return;
      }

      try {
        const invite = await this.auth.validateInvite(this.inviteToken);
        this.email = invite.email;
        this.emailReadonly.set(true);
      } catch {
        this.router.navigate(['/auth/login'], {
          queryParams: { message: 'invalid-invite' },
        });
        return;
      }

      this.initializing.set(false);
      this.cdr.detectChanges();
      this.tryAddPasswordIcons();
    } catch {
      this.error.set('Unable to check registration status. Please try again.');
      this.initializing.set(false);
      this.cdr.detectChanges();
      this.tryAddPasswordIcons();
    }
  }

  validateFirstName(): void {
    const errors = { ...this.fieldErrors() };
    errors.firstName = this.firstName.trim().length === 0 ? 'First name is required' : '';
    this.fieldErrors.set(errors);
  }

  validateLastName(): void {
    const errors = { ...this.fieldErrors() };
    errors.lastName = this.lastName.trim().length === 0 ? 'Last name is required' : '';
    this.fieldErrors.set(errors);
  }

  validateEmail(): void {
    const errors = { ...this.fieldErrors() };
    if (this.email.trim().length === 0) {
      errors.email = 'Email is required';
    } else if (!EMAIL_REGEX.test(this.email.trim())) {
      errors.email = 'Enter a valid email address';
    } else {
      errors.email = '';
    }
    this.fieldErrors.set(errors);
  }

  validateCompanyName(): void {
    const errors = { ...this.fieldErrors() };
    errors.companyName = this.companyName.trim().length === 0 ? 'Company name is required' : '';
    this.fieldErrors.set(errors);
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

  private validateAll(): boolean {
    this.validateFirstName();
    this.validateLastName();
    this.validateEmail();
    if (this.isOwnerRegistration()) {
      this.validateCompanyName();
    }

    const errs = this.fieldErrors();
    return !errs.firstName && !errs.lastName && !errs.email && (!this.isOwnerRegistration() || !errs.companyName);
  }

  async onSubmit(): Promise<void> {
    if (!this.validateAll()) {
      this.error.set('Please fix the errors above.');
      return;
    }
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
      await this.auth.signup(
        this.email,
        this.password,
        this.firstName.trim(),
        this.lastName.trim(),
        this.isOwnerRegistration() ? this.companyName.trim() : undefined,
        this.inviteToken ?? undefined,
      );
      this.router.navigate(['/auth/verify-otp'], { queryParams: { email: this.email } });
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Signup failed. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }
}
