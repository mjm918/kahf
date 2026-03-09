/**
 * Workspace management service.
 *
 * Handles workspace CRUD, listing, switching, and onboarding status
 * checks. Maintains the currently active workspace in a signal and
 * persists the selection in localStorage. Exposes reactive signals
 * for the current workspace and the full workspace list.
 *
 * WORKSPACE_COLORS — single source of truth for the curated color
 * palette used for workspace theming across the application (onboarding,
 * workspace switcher, settings).
 * DEFAULT_WORKSPACE_COLOR — default Azure blue used when no color is selected.
 * Workspace — interface for workspace data from the API.
 * WorkspaceService — injectable service managing workspace state.
 */

import { Injectable, signal, computed } from '@angular/core';
import { api } from './api.service';

export const WORKSPACE_COLORS: readonly string[] = [
  '#0078D4',
  '#0063B1',
  '#744DA9',
  '#B146C2',
  '#C239B3',
  '#E3008C',
  '#D13438',
  '#CA5010',
  '#EAA300',
  '#986F0B',
  '#498205',
  '#107C10',
  '#038387',
  '#005B70',
  '#394146',
  '#7A7574',
];

export const DEFAULT_WORKSPACE_COLOR = '#0078D4';

export interface Workspace {
  id: string;
  name: string;
  slug: string;
  color: string;
  created_by: string;
  created_at: string;
}

@Injectable({ providedIn: 'root' })
export class WorkspaceService {
  private readonly workspaces = signal<Workspace[]>([]);
  private readonly activeId = signal<string | null>(localStorage.getItem('active_workspace_id'));

  readonly list = this.workspaces.asReadonly();

  readonly current = computed(() => {
    const id = this.activeId();
    const all = this.workspaces();
    if (!id || all.length === 0) return null;
    return all.find(w => w.id === id) ?? all[0];
  });

  async loadWorkspaces(): Promise<Workspace[]> {
    const { data } = await api.get<Workspace[]>('/workspaces');
    this.workspaces.set(data);
    if (data.length > 0 && !this.activeId()) {
      this.switchWorkspace(data[0].id);
    }
    return data;
  }

  switchWorkspace(id: string): void {
    this.activeId.set(id);
    localStorage.setItem('active_workspace_id', id);
  }

  async createWorkspace(name: string, color: string): Promise<Workspace> {
    const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    const { data } = await api.post<Workspace>('/workspaces', { name, slug, color });
    const current = this.workspaces();
    this.workspaces.set([data, ...current]);
    this.switchWorkspace(data.id);
    return data;
  }

  async checkOnboardingNeeded(): Promise<boolean> {
    const { data } = await api.get<{ needs_onboarding: boolean }>('/workspaces/onboarding-status');
    return data.needs_onboarding;
  }

  async inviteToWorkspace(email: string): Promise<void> {
    await api.post('/auth/invite', { email });
  }
}
