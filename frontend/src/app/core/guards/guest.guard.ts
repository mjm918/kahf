/**
 * Route guard that redirects authenticated users away from auth pages.
 *
 * If the user is already logged in, navigates to the dashboard.
 * Used on login, signup, and verify-otp routes.
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from '../services/auth.service';

export const guestGuard: CanActivateFn = () => {
  const auth = inject(AuthService);
  const router = inject(Router);

  if (!auth.isAuthenticated()) {
    return true;
  }

  return router.createUrlTree(['/']);
};
