/**
 * Home page component displayed after login.
 *
 * Shows a welcome greeting with the authenticated user's name and a
 * quick-access grid of service shortcuts matching the Azure Portal
 * home layout pattern with compact card tiles. Includes a recent
 * activity placeholder section.
 */

import { Component, computed } from '@angular/core';
import { Router } from '@angular/router';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../../core/services/auth.service';

interface ServiceShortcut {
  id: string;
  name: string;
  iconCss: string;
  route: string;
}

@Component({
  selector: 'app-home',
  standalone: true,
  imports: [ButtonModule],
  template: `
    <div class="p-4 gap-6 flex flex-col">
      <div>
        <h1 class="text-base font-semibold mb-0">Welcome, {{ userName() }}!</h1>
        <p class="text-xs mb-4" style="color: var(--color-sf-content-text-color-alt2)">Your project management workspace</p>
      </div>

      <div>
        <h2 class="text-sm font-semibold mb-3">Services</h2>
        <div class="flex flex-wrap gap-6 mb-6">
          @for (svc of services; track svc.id) {
            <div class="flex flex-col items-center gap-1 cursor-pointer w-16" (click)="navigateTo(svc.route)">
              <span class="e-icons {{ svc.iconCss }} text-2xl" style="color: var(--color-sf-primary)"></span>
              <span class="text-xs text-center">{{ svc.name }}</span>
            </div>
          }
        </div>
      </div>

      <div>
        <h2 class="text-sm font-semibold mb-2">Resources</h2>
        <div class="flex gap-2 mb-3 text-xs">
          <span class="font-semibold" style="color: var(--color-sf-primary)">Recent</span>
          <span style="color: var(--color-sf-content-text-color-alt2)">Favorite</span>
        </div>
        <div class="e-card">
          <div class="e-card-content text-center py-6" style="color: var(--color-sf-content-text-color-alt2)">
            <p class="text-xs">No recent activity. Start by creating a task or document.</p>
          </div>
        </div>
      </div>
    </div>
  `,
})
export class Home {
  userName = computed(() => {
    const user = this.auth.currentUser();
    return user ? user.first_name : 'there';
  });

  services: ServiceShortcut[] = [
    { id: 'board', name: 'Board', iconCss: 'e-table', route: '/board' },
    { id: 'tasks', name: 'Tasks', iconCss: 'e-checklist', route: '/tasks' },
    { id: 'documents', name: 'Documents', iconCss: 'e-file-document', route: '/documents' },
    { id: 'chat', name: 'Chat', iconCss: 'e-comment-show', route: '/chat' },
    { id: 'calendar', name: 'Calendar', iconCss: 'e-month', route: '/calendar' },
    { id: 'contacts', name: 'Contacts', iconCss: 'e-people', route: '/contacts' },
    { id: 'drive', name: 'Drive', iconCss: 'e-folder', route: '/drive' },
  ];

  constructor(
    private auth: AuthService,
    private router: Router,
  ) {}

  navigateTo(route: string): void {
    this.router.navigate([route]);
  }
}
