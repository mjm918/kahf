/**
 * Top-level application routes.
 *
 * Auth pages (login, signup, verify-otp, forgot-password, reset-password)
 * are protected by the guest guard so authenticated users get redirected
 * to the dashboard. The onboarding route is protected by the auth guard
 * and shown to users who have no workspaces yet. The root path loads
 * the Shell layout (protected by auth + onboarding guards) which hosts
 * child routes for all application modules. Lazy-loaded where possible.
 */

import { Routes } from '@angular/router';
import { authGuard } from './core/guards/auth.guard';
import { guestGuard } from './core/guards/guest.guard';
import { onboardingGuard } from './core/guards/onboarding.guard';

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
    path: 'auth/forgot-password',
    canActivate: [guestGuard],
    loadComponent: () => import('./auth/forgot-password/forgot-password').then(m => m.ForgotPassword),
  },
  {
    path: 'auth/reset-password',
    canActivate: [guestGuard],
    loadComponent: () => import('./auth/reset-password/reset-password').then(m => m.ResetPassword),
  },
  {
    path: 'onboarding',
    canActivate: [authGuard],
    loadComponent: () => import('./onboarding/onboarding').then(m => m.Onboarding),
  },
  {
    path: '',
    canActivate: [authGuard, onboardingGuard],
    loadComponent: () => import('./layouts/shell').then(m => m.Shell),
    children: [
      { path: '', redirectTo: 'home', pathMatch: 'full' },
      {
        path: 'home',
        loadComponent: () => import('./modules/home/home').then(m => m.Home),
      },
    ],
  },
  {
    path: '**',
    redirectTo: '',
  },
];
