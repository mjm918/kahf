/**
 * Route guard that validates the user's session before allowing access.
 *
 * Awaits AuthService.ensureInitialized() which validates the stored
 * JWT against the backend on first call. If the token is invalid or
 * missing, redirects to /auth/login. This prevents the localStorage
 * bypass where a fake user JSON could fool a synchronous check.
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { AuthService } from '../services/auth.service';

export const authGuard: CanActivateFn = async () => {
  const auth = inject(AuthService);
  const router = inject(Router);

  await auth.ensureInitialized();

  if (auth.isAuthenticated()) {
    return true;
  }

  return router.createUrlTree(['/auth/login']);
};
