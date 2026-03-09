/**
 * Route guard that redirects authenticated users away from auth pages.
 *
 * Awaits AuthService.ensureInitialized() to confirm session validity
 * before checking. If the user is authenticated, redirects to the
 * dashboard. Used on login, signup, and verify-otp routes.
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from '../services/auth.service';

export const guestGuard: CanActivateFn = async () => {
  const auth = inject(AuthService);
  const router = inject(Router);

  await auth.ensureInitialized();

  if (!auth.isAuthenticated()) {
    return true;
  }

  return router.createUrlTree(['/']);
};
