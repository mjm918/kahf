/**
 * Root application component.
 *
 * Renders the router outlet which displays either auth pages or the
 * authenticated app shell based on the current route.
 */

import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet],
  template: '<router-outlet />',
})
export class App {}
