/**
 * Workspace management service.
 *
 * Handles workspace CRUD, listing, switching, member management, and
 * onboarding status checks. Maintains the currently active workspace
 * in a signal and persists the selection in localStorage. Exposes
 * reactive signals for the current workspace and the full workspace list.
 *
 * WORKSPACE_COLORS — single source of truth for the curated color
 * palette used for workspace theming across the application (onboarding,
 * workspace switcher, settings).
 * DEFAULT_WORKSPACE_COLOR — default Azure blue used when no color is selected.
 * Workspace — interface for workspace data from the API.
 * WorkspaceMember — interface for workspace member with user details.
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

export interface WorkspaceMember {
  user_id: string;
  email: string;
  first_name: string;
  last_name: string;
  avatar_url: string | null;
  role: string;
  joined_at: string;
}

export interface PendingInvitation {
  id: string;
  workspace_id: string;
  email: string;
  invited_by: string;
  expires_at: string;
  created_at: string;
}

export interface InviteResult {
  invitation_id: string | null;
  email: string;
  expires_at: string | null;
  added_directly: boolean;
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

  async updateWorkspace(id: string, body: { name?: string; color?: string }): Promise<Workspace> {
    const { data } = await api.patch<Workspace>(`/workspaces/${id}`, body);
    const current = this.workspaces();
    this.workspaces.set(current.map(w => w.id === id ? data : w));
    return data;
  }

  async deleteWorkspace(id: string): Promise<void> {
    await api.delete(`/workspaces/${id}`);
    const current = this.workspaces();
    this.workspaces.set(current.filter(w => w.id !== id));
    if (this.activeId() === id) {
      const remaining = this.workspaces();
      if (remaining.length > 0) {
        this.switchWorkspace(remaining[0].id);
      } else {
        this.activeId.set(null);
        localStorage.removeItem('active_workspace_id');
      }
    }
  }

  async listMembers(workspaceId: string): Promise<WorkspaceMember[]> {
    const { data } = await api.get<WorkspaceMember[]>(`/workspaces/${workspaceId}/members`);
    return data;
  }

  async removeMember(workspaceId: string, userId: string): Promise<void> {
    await api.delete(`/workspaces/${workspaceId}/members/${userId}`);
  }

  async updateMemberRole(workspaceId: string, userId: string, role: string): Promise<void> {
    await api.patch(`/workspaces/${workspaceId}/members/${userId}/role`, { role });
  }

  async inviteToWorkspace(email: string): Promise<InviteResult> {
    const wsId = this.current()?.id;
    if (!wsId) throw new Error('No active workspace');
    const { data } = await api.post<InviteResult>(`/workspaces/${wsId}/invitations`, { email });
    return data;
  }

  async listPendingInvitations(workspaceId: string): Promise<PendingInvitation[]> {
    const { data } = await api.get<PendingInvitation[]>(`/workspaces/${workspaceId}/invitations`);
    return data;
  }

  async cancelInvitation(workspaceId: string, id: string): Promise<void> {
    await api.delete(`/workspaces/${workspaceId}/invitations/${id}`);
  }
}
