/**
 * Application bootstrap entry point.
 *
 * Registers the Syncfusion license key (if set) and bootstraps the
 * root App component with the application configuration.
 */

import { bootstrapApplication } from '@angular/platform-browser';
import { registerLicense } from '@syncfusion/ej2-base';
import { appConfig } from './app/app.config';
import { App } from './app/app';

const SF_LICENSE = 'Ngo9BigBOggjHTQxAR8/V1JGaF5cXGpCf1FpRmJGdld5fUVHYVZUTXxaS00DNHVRdkdmWH1cdHRcQmhfVUB+XkFWYEs=';
if (SF_LICENSE) {
  registerLicense(SF_LICENSE);
}

bootstrapApplication(App, appConfig)
  .catch((err) => console.error(err));
