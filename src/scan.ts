import fs from 'fs-extra';
import { makeReport } from './features/report/index.js';
import type { createProgram } from './utils/cliArgs.js';
import { getErrorMessage, reportError } from './utils/error.js';
import { initLogger } from './utils/logger.js';
import { join } from './utils/path.js';

export const scan = async (
  options: ReturnType<typeof createProgram>['options'],
) => {
  try {
    initLogger('info', options.output !== 'console');
    const report = await makeReport(options.directory, options.ignore);

    if (options.output === 'file') {
      await fs.writeJSON(join(options.directory, 'scan-data.json'), report);
    } else {
      console.log(JSON.stringify(report));
    }
  } catch (e) {
    reportError({ message: getErrorMessage(e) });
    process.exit(1);
  }
};
