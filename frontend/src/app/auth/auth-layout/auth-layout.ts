/**
 * Shared auth layout with split-panel design.
 *
 * Left panel: Azure blue branding with Kahf logo, tagline, and feature list.
 * Right panel: ng-content slot for the auth form. Left panel hides on mobile.
 */

import { Component } from '@angular/core';

@Component({
  selector: 'app-auth-layout',
  standalone: true,
  templateUrl: './auth-layout.html',
})
export class AuthLayout {}
