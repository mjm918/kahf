/**
 * Workspace general settings page.
 *
 * Displays and allows editing the current workspace's name and color.
 * Provides a delete workspace action with confirmation dialog.
 *
 * WorkspaceGeneralSettings — standalone component for workspace configuration.
 */

import { Component, inject, signal, computed, OnInit } from '@angular/core';
import { Router } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { DialogModule } from '@syncfusion/ej2-angular-popups';
import { WorkspaceService, WORKSPACE_COLORS } from '../../core/services/workspace.service';

@Component({
  selector: 'app-workspace-general-settings',
  standalone: true,
  imports: [FormsModule, TextBoxModule, ButtonModule, MessageModule, DialogModule],
  templateUrl: './workspace-general.html',
})
export class WorkspaceGeneralSettings implements OnInit {
  private readonly workspaceService = inject(WorkspaceService);
  private readonly router = inject(Router);

  name = '';
  selectedColor = signal('');
  wsColors = WORKSPACE_COLORS;
  loading = signal(false);
  deleteLoading = signal(false);
  error = signal('');
  success = signal('');
  showDeleteDialog = signal(false);

  currentWorkspace = computed(() => this.workspaceService.current());

  ngOnInit(): void {
    const ws = this.workspaceService.current();
    if (ws) {
      this.name = ws.name;
      this.selectedColor.set(ws.color);
    }
  }

  selectColor(color: string): void {
    this.selectedColor.set(color);
  }

  async onSave(): Promise<void> {
    const ws = this.currentWorkspace();
    if (!ws) return;
    if (!this.name.trim()) {
      this.error.set('Workspace name is required.');
      return;
    }
    this.error.set('');
    this.success.set('');
    this.loading.set(true);
    try {
      await this.workspaceService.updateWorkspace(ws.id, {
        name: this.name.trim(),
        color: this.selectedColor(),
      });
      this.success.set('Workspace updated successfully.');
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to update workspace.');
    } finally {
      this.loading.set(false);
    }
  }

  confirmDelete(): void {
    this.showDeleteDialog.set(true);
  }

  async onDelete(): Promise<void> {
    const ws = this.currentWorkspace();
    if (!ws) return;
    this.showDeleteDialog.set(false);
    this.deleteLoading.set(true);
    try {
      await this.workspaceService.deleteWorkspace(ws.id);
      this.router.navigate(['/']);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to delete workspace.');
    } finally {
      this.deleteLoading.set(false);
    }
  }
}
