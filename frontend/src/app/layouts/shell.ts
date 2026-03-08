/**
 * Application shell layout implementing Azure Portal design language.
 *
 * Provides the primary authenticated layout with a compact sticky top
 * AppBar (hamburger toggle, branding, pill-shaped search bar, icon buttons
 * for notifications/settings/help, and a user avatar that opens an Azure-style
 * profile popup), a collapsible left Sidebar in overlay mode with flat
 * navigation lists organized by section headers, and a main content area
 * hosting the router outlet for child module pages.
 *
 * NavItem — navigation item descriptor with id, name, icon class, and route.
 * Shell — root layout component managing sidebar toggle, navigation routing,
 *         user panel visibility, and computed user display properties.
 */

import { Component, ViewChild, computed, signal, HostListener } from '@angular/core';
import { Router, RouterOutlet } from '@angular/router';
import {
  AppBarModule,
  SidebarComponent,
  SidebarModule,
} from '@syncfusion/ej2-angular-navigations';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { AuthService } from '../core/services/auth.service';

interface NavItem {
  id: string;
  name: string;
  iconCss: string;
  route: string;
}

@Component({
  selector: 'app-shell',
  standalone: true,
  imports: [
    RouterOutlet,
    AppBarModule,
    SidebarModule,
    ButtonModule,
  ],
  templateUrl: './shell.html',
})
export class Shell {
  @ViewChild('sidebar') sidebar!: SidebarComponent;

  userPanelOpen = signal(false);

  userInitials = computed(() => {
    const user = this.auth.currentUser();
    if (!user) return '?';
    return `${user.first_name?.[0] ?? ''}${user.last_name?.[0] ?? ''}`.toUpperCase();
  });

  userDisplayName = computed(() => {
    const user = this.auth.currentUser();
    if (!user) return '';
    return `${user.first_name} ${user.last_name}`.trim();
  });

  topNavItems: NavItem[] = [
    { id: 'home', name: 'Home', iconCss: 'e-icons e-home', route: '/home' },
    { id: 'dashboard', name: 'Dashboard', iconCss: 'e-icons e-grid-view', route: '/dashboard' },
  ];

  moduleNavItems: NavItem[] = [
    { id: 'board', name: 'Board', iconCss: 'e-icons e-table', route: '/board' },
    { id: 'tasks', name: 'Tasks', iconCss: 'e-icons e-checklist', route: '/tasks' },
    { id: 'documents', name: 'Documents', iconCss: 'e-icons e-file-document', route: '/documents' },
    { id: 'chat', name: 'Chat', iconCss: 'e-icons e-comment-show', route: '/chat' },
    { id: 'calendar', name: 'Calendar', iconCss: 'e-icons e-month', route: '/calendar' },
    { id: 'contacts', name: 'Contacts', iconCss: 'e-icons e-people', route: '/contacts' },
    { id: 'hr', name: 'HR', iconCss: 'e-icons e-user', route: '/hr' },
    { id: 'drive', name: 'Drive', iconCss: 'e-icons e-folder', route: '/drive' },
  ];

  settingsItem: NavItem = { id: 'settings', name: 'Settings', iconCss: 'e-icons e-settings', route: '/settings' };

  constructor(
    public auth: AuthService,
    private router: Router,
  ) {}

  toggleSidebar(): void {
    this.sidebar.toggle();
  }

  onNavItemClick(item: NavItem): void {
    this.sidebar.hide();
    this.router.navigate([item.route]);
  }

  toggleUserPanel(event: MouseEvent): void {
    event.stopPropagation();
    this.userPanelOpen.update(v => !v);
  }

  @HostListener('document:click')
  closeUserPanel(): void {
    if (this.userPanelOpen()) {
      this.userPanelOpen.set(false);
    }
  }
}
