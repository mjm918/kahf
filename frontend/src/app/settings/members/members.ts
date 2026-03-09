/**
 * Workspace members management page.
 *
 * Displays workspace members with role, email, and actions (change role,
 * remove). Shows pending invitations with re-invite and cancel buttons.
 * Provides an invite member form to send workspace invitations by email.
 * If the invited user already has an account, they are added to the
 * workspace directly as a member.
 *
 * MemberSettings — standalone component for workspace member management.
 */

import { Component, inject, signal, computed, OnInit } from '@angular/core';
import { DatePipe } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { GridModule, PageService, SortService, FilterService } from '@syncfusion/ej2-angular-grids';
import { TextBoxModule } from '@syncfusion/ej2-angular-inputs';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { DropDownListModule } from '@syncfusion/ej2-angular-dropdowns';
import { MessageModule, ToastModule } from '@syncfusion/ej2-angular-notifications';
import { DialogModule } from '@syncfusion/ej2-angular-popups';
import { WorkspaceService, WorkspaceMember, PendingInvitation } from '../../core/services/workspace.service';
import { AuthService } from '../../core/services/auth.service';

@Component({
  selector: 'app-member-settings',
  standalone: true,
  imports: [
    DatePipe,
    FormsModule,
    GridModule,
    TextBoxModule,
    ButtonModule,
    DropDownListModule,
    MessageModule,
    ToastModule,
    DialogModule,
  ],
  providers: [PageService, SortService, FilterService],
  templateUrl: './members.html',
})
export class MemberSettings implements OnInit {
  private readonly workspaceService = inject(WorkspaceService);
  private readonly auth = inject(AuthService);

  members = signal<WorkspaceMember[]>([]);
  pendingInvites = signal<PendingInvitation[]>([]);
  loading = signal(true);
  error = signal('');
  success = signal('');
  inviteEmail = '';
  inviteLoading = signal(false);
  inviteError = signal('');
  showRemoveDialog = signal(false);
  memberToRemove = signal<WorkspaceMember | null>(null);
  removeLoading = signal(false);
  reinviteLoadingId = signal<string | null>(null);
  cancelInviteLoadingId = signal<string | null>(null);
  showCancelInviteDialog = signal(false);
  now = new Date().toISOString();
  inviteToCancel = signal<PendingInvitation | null>(null);

  currentUserId = computed(() => this.auth.currentUser()?.user_id ?? '');
  workspaceId = computed(() => this.workspaceService.current()?.id ?? '');

  roleOptions = [
    { text: 'Owner', value: 'owner' },
    { text: 'Admin', value: 'admin' },
    { text: 'Member', value: 'member' },
    { text: 'Viewer', value: 'viewer' },
  ];

  async ngOnInit(): Promise<void> {
    await this.loadAll();
  }

  async loadAll(): Promise<void> {
    const wsId = this.workspaceId();
    if (!wsId) return;
    this.loading.set(true);
    try {
      const [members, invites] = await Promise.all([
        this.workspaceService.listMembers(wsId),
        this.workspaceService.listPendingInvitations(wsId),
      ]);
      this.members.set(members);
      this.pendingInvites.set(invites);
    } catch {
      this.error.set('Failed to load members.');
    } finally {
      this.loading.set(false);
    }
  }

  async onRoleChange(member: WorkspaceMember, newRole: string): Promise<void> {
    const wsId = this.workspaceId();
    if (!wsId || newRole === member.role) return;
    try {
      await this.workspaceService.updateMemberRole(wsId, member.user_id, newRole);
      this.members.update(list =>
        list.map(m => m.user_id === member.user_id ? { ...m, role: newRole } : m)
      );
      this.success.set(`Role updated for ${member.first_name} ${member.last_name}.`);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to update role.');
    }
  }

  confirmRemove(member: WorkspaceMember): void {
    this.memberToRemove.set(member);
    this.showRemoveDialog.set(true);
  }

  async onRemove(): Promise<void> {
    const wsId = this.workspaceId();
    const member = this.memberToRemove();
    if (!wsId || !member) return;
    this.showRemoveDialog.set(false);
    this.removeLoading.set(true);
    try {
      await this.workspaceService.removeMember(wsId, member.user_id);
      this.members.update(list => list.filter(m => m.user_id !== member.user_id));
      this.success.set(`${member.first_name} ${member.last_name} removed.`);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to remove member.');
    } finally {
      this.removeLoading.set(false);
      this.memberToRemove.set(null);
    }
  }

  async onInvite(): Promise<void> {
    const email = this.inviteEmail.trim();
    if (!email) {
      this.inviteError.set('Email is required.');
      return;
    }
    this.inviteError.set('');
    this.inviteLoading.set(true);
    try {
      const result = await this.workspaceService.inviteToWorkspace(email);
      if (result.added_directly) {
        this.success.set(`${email} added to workspace.`);
        const wsId = this.workspaceId();
        if (wsId) {
          const members = await this.workspaceService.listMembers(wsId);
          this.members.set(members);
        }
      } else {
        this.success.set(`Invitation sent to ${email}.`);
        const wsId = this.workspaceId();
        if (wsId) {
          const invites = await this.workspaceService.listPendingInvitations(wsId);
          this.pendingInvites.set(invites);
        }
      }
      this.inviteEmail = '';
    } catch (err: any) {
      this.inviteError.set(err?.response?.data?.error || 'Failed to send invitation.');
    } finally {
      this.inviteLoading.set(false);
    }
  }

  async reInvite(invite: PendingInvitation): Promise<void> {
    this.reinviteLoadingId.set(invite.id);
    try {
      await this.workspaceService.inviteToWorkspace(invite.email);
      this.success.set(`Invitation re-sent to ${invite.email}.`);
      const wsId = this.workspaceId();
      if (wsId) {
        const invites = await this.workspaceService.listPendingInvitations(wsId);
        this.pendingInvites.set(invites);
      }
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to re-send invitation.');
    } finally {
      this.reinviteLoadingId.set(null);
    }
  }

  confirmCancelInvite(invite: PendingInvitation): void {
    this.inviteToCancel.set(invite);
    this.showCancelInviteDialog.set(true);
  }

  async onCancelInvite(): Promise<void> {
    const wsId = this.workspaceId();
    const invite = this.inviteToCancel();
    if (!wsId || !invite) return;
    this.showCancelInviteDialog.set(false);
    this.cancelInviteLoadingId.set(invite.id);
    try {
      await this.workspaceService.cancelInvitation(wsId, invite.id);
      this.pendingInvites.update(list => list.filter(i => i.id !== invite.id));
      this.success.set(`Invitation to ${invite.email} cancelled.`);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to cancel invitation.');
    } finally {
      this.cancelInviteLoadingId.set(null);
      this.inviteToCancel.set(null);
    }
  }

  memberName(member: WorkspaceMember): string {
    return `${member.first_name} ${member.last_name}`.trim();
  }

  isOwner(member: WorkspaceMember): boolean {
    return member.role === 'owner';
  }

  canChangeRole(member: WorkspaceMember): boolean {
    if (member.user_id === this.currentUserId()) return false;
    if (member.role === 'owner') {
      const ownerCount = this.members().filter(m => m.role === 'owner').length;
      return ownerCount > 1;
    }
    return true;
  }

  canRemove(member: WorkspaceMember): boolean {
    if (member.user_id === this.currentUserId()) return false;
    if (member.role === 'owner') return false;
    return true;
  }
}
