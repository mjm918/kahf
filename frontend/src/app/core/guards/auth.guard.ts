/**
 * Route guard that redirects unauthenticated users to /auth/login.
 *
 * Checks the AuthService.isAuthenticated signal. If false, navigates
 * to the login page and blocks activation. Used on all protected
 * routes inside the app shell.
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from '../services/auth.service';

export const authGuard: CanActivateFn = () => {
  const auth = inject(AuthService);
  const router = inject(Router);

  if (auth.isAuthenticated()) {
    return true;
  }

  return router.createUrlTree(['/auth/login']);
};
