/**
 * Post-signup onboarding wizard component.
 *
 * Two-step flow presented after a new user completes OTP verification:
 * Step 1 — Create workspace: enter a workspace name and pick a custom
 * color from a curated palette. Submits to WorkspaceService.createWorkspace.
 * Step 2 — Invite team members: add email addresses one at a time and
 * send invitations. Users can skip this step. On completion, navigates
 * to the main app shell.
 *
 * WORKSPACE_COLORS — curated color palette for workspace theming.
 * Onboarding — standalone component managing the wizard state machine.
 */

import { Component, signal } from '@angular/core';
import { Router } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule, ChipListModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule, ToastModule } from '@syncfusion/ej2-angular-notifications';
import { WorkspaceService, WORKSPACE_COLORS, DEFAULT_WORKSPACE_COLOR } from '../core/services/workspace.service';
import { AuthService } from '../core/services/auth.service';

@Component({
  selector: 'app-onboarding',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, ChipListModule, MessageModule, ToastModule],
  templateUrl: './onboarding.html',
})
export class Onboarding {
  step = signal<1 | 2>(1);
  workspaceName = '';
  selectedColor = signal(DEFAULT_WORKSPACE_COLOR);
  colors = WORKSPACE_COLORS;
  error = signal('');
  loading = signal(false);

  inviteEmail = '';
  invitedEmails = signal<string[]>([]);
  inviteError = signal('');
  inviteLoading = signal(false);

  userName = '';

  constructor(
    private workspace: WorkspaceService,
    private auth: AuthService,
    private router: Router,
  ) {
    const user = this.auth.currentUser();
    this.userName = user?.first_name ?? '';
  }

  selectColor(color: string): void {
    this.selectedColor.set(color);
  }

  async onCreateWorkspace(): Promise<void> {
    const name = this.workspaceName.trim();
    if (!name) {
      this.error.set('Workspace name is required.');
      return;
    }
    if (name.length < 2) {
      this.error.set('Workspace name must be at least 2 characters.');
      return;
    }
    this.error.set('');
    this.loading.set(true);
    try {
      await this.workspace.createWorkspace(name, this.selectedColor());
      this.step.set(2);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to create workspace.');
    } finally {
      this.loading.set(false);
    }
  }

  async onInvite(): Promise<void> {
    const email = this.inviteEmail.trim();
    if (!email) return;
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      this.inviteError.set('Enter a valid email address.');
      return;
    }
    if (this.invitedEmails().includes(email)) {
      this.inviteError.set('This email has already been invited.');
      return;
    }
    this.inviteError.set('');
    this.inviteLoading.set(true);
    try {
      await this.workspace.inviteToWorkspace(email);
      this.invitedEmails.update(list => [...list, email]);
      this.inviteEmail = '';
    } catch (err: any) {
      this.inviteError.set(err?.response?.data?.error || 'Failed to send invitation.');
    } finally {
      this.inviteLoading.set(false);
    }
  }

  onFinish(): void {
    this.router.navigate(['/']);
  }
}
