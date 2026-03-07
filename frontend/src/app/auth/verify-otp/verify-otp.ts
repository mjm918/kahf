/**
 * OTP verification page component.
 *
 * Reads the email from query params. Displays a Syncfusion OtpInput
 * (6 digits) for the user to enter the verification code sent to their
 * email. On submit, calls AuthService.verifyOtp and navigates to the
 * dashboard. Provides a resend button with a 60-second cooldown timer.
 */

import { Component, OnInit, OnDestroy, signal } from '@angular/core';
import { Router, ActivatedRoute } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { OtpInputModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../../core/services/auth.service';
import { AuthLayout } from '../auth-layout/auth-layout';

@Component({
  selector: 'app-verify-otp',
  standalone: true,
  imports: [FormsModule, OtpInputModule, ButtonModule, AuthLayout],
  templateUrl: './verify-otp.html',
})
export class VerifyOtp implements OnInit, OnDestroy {
  email = '';
  otpValue = '';
  error = signal('');
  success = signal('');
  loading = signal(false);
  resendCooldown = signal(0);

  private timerId: ReturnType<typeof setInterval> | null = null;

  constructor(
    private auth: AuthService,
    private router: Router,
    private route: ActivatedRoute,
  ) {}

  ngOnInit(): void {
    this.email = this.route.snapshot.queryParamMap.get('email') ?? '';
    if (!this.email) {
      this.router.navigate(['/auth/signup']);
    }
  }

  ngOnDestroy(): void {
    if (this.timerId) clearInterval(this.timerId);
  }

  onOtpChange(event: any): void {
    this.otpValue = event.value?.toString() ?? '';
  }

  async onVerify(): Promise<void> {
    if (this.otpValue.length < 6) {
      this.error.set('Please enter the full 6-digit code.');
      return;
    }
    this.error.set('');
    this.loading.set(true);
    try {
      await this.auth.verifyOtp(this.email, this.otpValue);
      this.router.navigate(['/']);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Invalid or expired code. Please try again.');
    } finally {
      this.loading.set(false);
    }
  }

  async onResend(): Promise<void> {
    this.error.set('');
    this.success.set('');
    try {
      await this.auth.resendOtp(this.email);
      this.success.set('A new code has been sent to your email.');
      this.startCooldown();
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Could not resend code.');
    }
  }

  private startCooldown(): void {
    this.resendCooldown.set(60);
    if (this.timerId) clearInterval(this.timerId);
    this.timerId = setInterval(() => {
      const val = this.resendCooldown();
      if (val <= 1) {
        this.resendCooldown.set(0);
        if (this.timerId) clearInterval(this.timerId);
      } else {
        this.resendCooldown.set(val - 1);
      }
    }, 1000);
  }
}
