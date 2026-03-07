/**
 * Application shell layout placeholder.
 *
 * Displays the main authenticated layout with sidebar navigation,
 * top bar, and content area. Currently shows a welcome message.
 * Will be built out with Azure Portal-style layout in subsequent work.
 */

import { Component } from '@angular/core';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../core/services/auth.service';

@Component({
  selector: 'app-shell',
  standalone: true,
  imports: [ButtonModule],
  template: `
    <div class="flex items-center justify-center min-h-screen">
      <div class="e-card max-w-md w-full">
        <div class="e-card-header">
          <div class="e-card-header-title text-center">
            <span class="text-xl font-semibold" style="color: var(--color-sf-primary)">Welcome to Kahf</span>
          </div>
        </div>
        <div class="e-card-content text-center">
          <div class="mb-4" style="color: var(--color-sf-content-text-color-alt2)">
            You are signed in as <strong>{{ auth.currentUser()?.email }}</strong>
          </div>
          <button ejs-button [isPrimary]="false" content="Sign out" (click)="auth.logout()"></button>
        </div>
      </div>
    </div>
  `,
})
export class Shell {
  constructor(public auth: AuthService) {}
}
