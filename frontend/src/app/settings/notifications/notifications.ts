/**
 * Notification preferences settings page.
 *
 * Displays per-channel notification toggles (Email, Telegram, Web Push,
 * In-App) with snooze controls. Loads current preferences on init and
 * updates them via the notification service.
 *
 * NotificationSettings — standalone component for notification preferences.
 */

import { Component, inject, signal, OnInit } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ButtonModule, SwitchModule } from '@syncfusion/ej2-angular-buttons';
import { MessageModule } from '@syncfusion/ej2-angular-notifications';
import { NotificationService, NotificationPreference } from '../../core/services/notification.service';

interface ChannelCard {
  channel: string;
  label: string;
  description: string;
  icon: string;
  color: string;
  enabled: boolean;
  snoozedUntil: string | null;
}

@Component({
  selector: 'app-notification-settings',
  standalone: true,
  imports: [FormsModule, ButtonModule, SwitchModule, MessageModule],
  templateUrl: './notifications.html',
})
export class NotificationSettings implements OnInit {
  private readonly notificationService = inject(NotificationService);

  channels = signal<ChannelCard[]>([]);
  loading = signal(true);
  error = signal('');

  private readonly channelMeta: Record<string, { label: string; description: string; icon: string; color: string }> = {
    email: { label: 'Email', description: 'Receive notifications via email', icon: '@', color: '#0078D4' },
    telegram: { label: 'Telegram', description: 'Receive notifications via Telegram bot', icon: 'T', color: '#0088CC' },
    web_push: { label: 'Browser Push', description: 'Receive browser push notifications', icon: 'P', color: '#744DA9' },
    in_app: { label: 'In-App', description: 'Receive in-app notification bell alerts', icon: 'N', color: '#CA5010' },
  };

  async ngOnInit(): Promise<void> {
    try {
      const prefs = await this.notificationService.getPreferences();
      const allChannels = ['email', 'telegram', 'web_push', 'in_app'];
      const cards: ChannelCard[] = allChannels.map(ch => {
        const pref = prefs.find(p => p.channel === ch);
        const meta = this.channelMeta[ch];
        return {
          channel: ch,
          label: meta.label,
          description: meta.description,
          icon: meta.icon,
          color: meta.color,
          enabled: pref?.enabled ?? true,
          snoozedUntil: pref?.snoozed_until ?? null,
        };
      });
      this.channels.set(cards);
    } catch {
      this.error.set('Failed to load notification preferences.');
    } finally {
      this.loading.set(false);
    }
  }

  async onToggle(card: ChannelCard): Promise<void> {
    const newEnabled = !card.enabled;
    try {
      await this.notificationService.setPreference(card.channel, newEnabled);
      this.channels.update(cards =>
        cards.map(c => c.channel === card.channel ? { ...c, enabled: newEnabled } : c)
      );
    } catch {
      this.error.set(`Failed to update ${card.label} preference.`);
    }
  }
}
