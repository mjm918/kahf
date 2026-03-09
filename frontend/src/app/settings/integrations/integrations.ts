/**
 * Integrations settings page.
 *
 * Displays available integrations (Telegram) with link/unlink actions.
 * For Telegram, generates a 6-character link code and shows instructions
 * for sending /link CODE to the bot. Polls link status while code is
 * active and shows connected state when linked.
 *
 * IntegrationSettings — standalone component for managing integrations.
 */

import { Component, inject, signal, OnInit, OnDestroy } from '@angular/core';
import { ButtonModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { DialogModule } from '@syncfusion/ej2-angular-popups';
import { NotificationService, TelegramLinkStatus, TelegramLinkCode } from '../../core/services/notification.service';

@Component({
  selector: 'app-integration-settings',
  standalone: true,
  imports: [ButtonModule, MessageModule, DialogModule],
  templateUrl: './integrations.html',
})
export class IntegrationSettings implements OnInit, OnDestroy {
  private readonly notificationService = inject(NotificationService);
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  telegramStatus = signal<TelegramLinkStatus | null>(null);
  linkCode = signal<TelegramLinkCode | null>(null);
  loading = signal(false);
  unlinkLoading = signal(false);
  error = signal('');
  showUnlinkDialog = signal(false);

  async ngOnInit(): Promise<void> {
    await this.loadTelegramStatus();
  }

  ngOnDestroy(): void {
    this.stopPolling();
  }

  async loadTelegramStatus(): Promise<void> {
    try {
      const status = await this.notificationService.getTelegramStatus();
      this.telegramStatus.set(status);
    } catch {
      this.telegramStatus.set({ linked: false, telegram_username: null });
    }
  }

  async generateCode(): Promise<void> {
    this.error.set('');
    this.loading.set(true);
    try {
      const code = await this.notificationService.generateTelegramCode();
      this.linkCode.set(code);
      this.startPolling();
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to generate link code.');
    } finally {
      this.loading.set(false);
    }
  }

  confirmUnlink(): void {
    this.showUnlinkDialog.set(true);
  }

  async onUnlink(): Promise<void> {
    this.showUnlinkDialog.set(false);
    this.unlinkLoading.set(true);
    try {
      await this.notificationService.unlinkTelegram();
      this.telegramStatus.set({ linked: false, telegram_username: null });
      this.linkCode.set(null);
    } catch (err: any) {
      this.error.set(err?.response?.data?.error || 'Failed to unlink Telegram.');
    } finally {
      this.unlinkLoading.set(false);
    }
  }

  private startPolling(): void {
    this.stopPolling();
    this.pollTimer = setInterval(async () => {
      try {
        const status = await this.notificationService.getTelegramStatus();
        if (status.linked) {
          this.telegramStatus.set(status);
          this.linkCode.set(null);
          this.stopPolling();
        }
      } catch {
        /* ignore polling errors */
      }
    }, 3000);
  }

  private stopPolling(): void {
    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
  }
}
