import { scan } from './scan.js';
import { createProgram } from './utils/cliArgs.js';

export const run = async () => {
  const result = createProgram();

  if (result.command === 'scan') {
    scan(result.options);
  }
};
