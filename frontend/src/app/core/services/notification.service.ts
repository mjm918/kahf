/**
 * Notification preferences and in-app notification management service.
 *
 * Handles notification preference CRUD (per-channel enable/disable,
 * snooze/unsnooze), in-app notification listing with pagination,
 * unread count, mark read, and push subscription management.
 *
 * NotificationPreference — per-channel preference state.
 * InAppNotification — in-app notification payload.
 * NotificationService — injectable service for notification operations.
 */

import { Injectable, signal } from '@angular/core';
import { api } from './api.service';

export interface NotificationPreference {
  channel: string;
  enabled: boolean;
  snoozed_until: string | null;
}

export interface InAppNotification {
  id: string;
  title: string;
  body: string | null;
  category: string | null;
  read: boolean;
  created_at: string;
}

export interface TelegramLinkStatus {
  linked: boolean;
  telegram_username: string | null;
}

export interface TelegramLinkCode {
  code: string;
  bot_username: string;
  expires_in_minutes: number;
}

@Injectable({ providedIn: 'root' })
export class NotificationService {
  readonly unreadCount = signal(0);

  async getPreferences(): Promise<NotificationPreference[]> {
    const { data } = await api.get<NotificationPreference[]>('/notifications/preferences');
    return data;
  }

  async setPreference(channel: string, enabled: boolean): Promise<void> {
    await api.put(`/notifications/preferences/${channel}`, { enabled });
  }

  async snoozeChannel(channel: string, minutes: number): Promise<void> {
    await api.post(`/notifications/preferences/${channel}/snooze`, { minutes });
  }

  async unsnoozeChannel(channel: string): Promise<void> {
    await api.post(`/notifications/preferences/${channel}/unsnooze`);
  }

  async snoozeAll(minutes: number): Promise<void> {
    await api.post('/notifications/preferences/snooze-all', { minutes });
  }

  async unsnoozeAll(): Promise<void> {
    await api.post('/notifications/preferences/unsnooze-all');
  }

  async getNotifications(limit: number = 20, offset: number = 0): Promise<InAppNotification[]> {
    const { data } = await api.get<InAppNotification[]>(`/notifications?limit=${limit}&offset=${offset}`);
    return data;
  }

  async getUnreadCount(): Promise<number> {
    const { data } = await api.get<{ count: number }>('/notifications/unread-count');
    this.unreadCount.set(data.count);
    return data.count;
  }

  async markRead(id: string): Promise<void> {
    await api.post(`/notifications/${id}/read`);
    this.unreadCount.update(c => Math.max(0, c - 1));
  }

  async markAllRead(): Promise<void> {
    await api.post('/notifications/read-all');
    this.unreadCount.set(0);
  }

  async getTelegramStatus(): Promise<TelegramLinkStatus> {
    const { data } = await api.get<TelegramLinkStatus>('/telegram/link');
    return data;
  }

  async generateTelegramCode(): Promise<TelegramLinkCode> {
    const { data } = await api.post<TelegramLinkCode>('/telegram/link');
    return data;
  }

  async unlinkTelegram(): Promise<void> {
    await api.delete('/telegram/link');
  }
}
