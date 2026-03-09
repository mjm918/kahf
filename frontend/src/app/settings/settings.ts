/**
 * Settings layout component with Huly-style left navigation sidebar.
 *
 * Renders a two-panel layout: a fixed left navigation sidebar listing
 * user-level and workspace-level settings sections, and a right
 * content area hosting child routes via router outlet. Bottom section
 * includes workspace actions (select workspace, invite, sign out).
 *
 * SettingsNavItem — navigation item descriptor with label, icon, and route.
 * Settings — layout component managing settings navigation and active state.
 */

import { Component, computed, inject } from '@angular/core';
import { Router, RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../core/services/auth.service';
import { WorkspaceService } from '../core/services/workspace.service';

interface SettingsNavItem {
  label: string;
  iconCss: string;
  route: string;
}

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [RouterOutlet, RouterLink, RouterLinkActive, ButtonModule],
  templateUrl: './settings.html',
})
export class Settings {
  private readonly auth = inject(AuthService);
  private readonly workspaceService = inject(WorkspaceService);
  private readonly router = inject(Router);

  readonly currentWorkspace = computed(() => this.workspaceService.current());

  readonly userSettings: SettingsNavItem[] = [
    { label: 'Account settings', iconCss: 'e-icons e-user', route: '/settings/account' },
    { label: 'Change password', iconCss: 'e-icons e-lock', route: '/settings/password' },
    { label: 'Integrations', iconCss: 'e-icons e-link', route: '/settings/integrations' },
    { label: 'Notifications', iconCss: 'e-icons e-comment-show', route: '/settings/notifications' },
  ];

  readonly workspaceSettings: SettingsNavItem[] = [
    { label: 'General', iconCss: 'e-icons e-settings', route: '/settings/workspace' },
    { label: 'Members', iconCss: 'e-icons e-people', route: '/settings/members' },
  ];

  goBack(): void {
    this.router.navigate(['/home']);
  }

  logout(): void {
    this.auth.logout();
  }
}
