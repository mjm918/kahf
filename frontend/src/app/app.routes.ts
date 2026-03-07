/**
 * Top-level application routes.
 *
 * Auth pages (login, signup, verify-otp) are protected by the guest
 * guard so authenticated users get redirected to the dashboard. The
 * root path is protected by the auth guard. Lazy-loaded where possible.
 */

import { Routes } from '@angular/router';
import { authGuard } from './core/guards/auth.guard';
import { guestGuard } from './core/guards/guest.guard';

export const routes: Routes = [
  {
    path: 'auth/login',
    canActivate: [guestGuard],
    loadComponent: () => import('./auth/login/login').then(m => m.Login),
  },
  {
    path: 'auth/signup',
    canActivate: [guestGuard],
    loadComponent: () => import('./auth/signup/signup').then(m => m.Signup),
  },
  {
    path: 'auth/verify-otp',
    canActivate: [guestGuard],
    loadComponent: () => import('./auth/verify-otp/verify-otp').then(m => m.VerifyOtp),
  },
  {
    path: '',
    canActivate: [authGuard],
    loadComponent: () => import('./layouts/shell').then(m => m.Shell),
  },
  {
    path: '**',
    redirectTo: '',
  },
];
