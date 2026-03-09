/**
 * Reusable password strength meter component.
 *
 * Evaluates password against complexity checks and displays four
 * colored bar segments representing Weak, Fair, Good, and Strong
 * levels. Each segment fills when its threshold is reached. All
 * active segments share the color of the current level. Individual
 * check indicators show which rules pass.
 *
 * PASSWORD_CHECKS — default complexity rules (8+ chars, uppercase,
 *   lowercase, number, special character).
 * PasswordCheck — interface for a single check rule with label and test fn.
 * PasswordStrengthMeter — standalone component accepting password input
 *   and emitting strength percentage.
 */

import { Component, input, computed, output } from '@angular/core';

export interface PasswordCheck {
  label: string;
  test: (pw: string) => boolean;
}

export const PASSWORD_CHECKS: PasswordCheck[] = [
  { label: '8+ characters', test: (pw) => pw.length >= 8 },
  { label: 'Uppercase letter', test: (pw) => /[A-Z]/.test(pw) },
  { label: 'Lowercase letter', test: (pw) => /[a-z]/.test(pw) },
  { label: 'Number', test: (pw) => /\d/.test(pw) },
  { label: 'Special character', test: (pw) => /[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(pw) },
];

const LEVEL_COLORS = ['#D13438', '#CA5010', '#0078D4', '#107C10'];
const LEVEL_LABELS = ['Weak', 'Fair', 'Good', 'Strong'];
const LEVEL_MIN_CHECKS = [1, 2, 3, 5];

@Component({
  selector: 'app-password-strength',
  standalone: true,
  templateUrl: './password-strength.html',
})
export class PasswordStrengthMeter {
  readonly password = input.required<string>();
  readonly strengthChange = output<number>();

  readonly checks = PASSWORD_CHECKS;
  readonly barIndices = [0, 1, 2, 3];

  readonly checkResults = computed(() =>
    this.checks.map(c => c.test(this.password()))
  );

  readonly passedCount = computed(() =>
    this.checkResults().filter(Boolean).length
  );

  readonly strength = computed(() => {
    const pct = (this.passedCount() / this.checks.length) * 100;
    this.strengthChange.emit(pct);
    return pct;
  });

  readonly activeLevel = computed(() => {
    const pw = this.password();
    if (pw.length === 0) return -1;
    const passed = this.passedCount();
    let active = -1;
    for (let i = 0; i < LEVEL_MIN_CHECKS.length; i++) {
      if (passed >= LEVEL_MIN_CHECKS[i]) active = i;
    }
    return active;
  });

  readonly label = computed(() => {
    const s = this.strength();
    const idx = this.activeLevel();
    return idx >= 0 ? LEVEL_LABELS[idx] : '';
  });

  readonly labelColor = computed(() => {
    const idx = this.activeLevel();
    return idx >= 0 ? LEVEL_COLORS[idx] : '#D2D0CE';
  });

  readonly bars = computed(() => {
    const active = this.activeLevel();
    const color = active >= 0 ? LEVEL_COLORS[active] : '#EDEBE9';
    return this.barIndices.map(i => ({
      active: i <= active,
      color: i <= active ? color : '#EDEBE9',
    }));
  });

  readonly visible = computed(() => this.password().length > 0);
}
