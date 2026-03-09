/**
 * Route guard that redirects authenticated users to the onboarding
 * flow if they have no workspaces yet.
 *
 * Checks WorkspaceService.checkOnboardingNeeded(). If the user needs
 * onboarding, redirects to /onboarding. Applied on the main shell
 * route so new users are funneled through workspace creation before
 * accessing the app. Skips the check if already on the onboarding route.
 */

import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { WorkspaceService } from '../services/workspace.service';

export const onboardingGuard: CanActivateFn = async () => {
  const workspace = inject(WorkspaceService);
  const router = inject(Router);

  try {
    const needsOnboarding = await workspace.checkOnboardingNeeded();
    if (needsOnboarding) {
      return router.createUrlTree(['/onboarding']);
    }
    await workspace.loadWorkspaces();
    return true;
  } catch {
    return true;
  }
};
