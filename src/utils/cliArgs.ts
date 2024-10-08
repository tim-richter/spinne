import yargs, { type CommandModule } from 'yargs';
import { hideBin } from 'yargs/helpers';
import { cwd } from './path.js';

type Return = {
  command: 'scan';
  options: {
    o: 'file' | 'console';
    output: 'file' | 'console';
    d: string;
    directory: string;
    ignore: Array<string>;
    i: Array<string>;
  };
};

export const createProgram = (): Return => {
  const scanCommand: CommandModule = {
    command: 'scan',
    describe: 'Scan a directory for components',
    builder: {
      directory: {
        alias: 'd',
        describe: 'Path to a different directory',
        type: 'string',
        default: cwd(),
      },
      output: {
        alias: 'o',
        describe: 'Output format',
        type: 'string',
        choices: ['file', 'console'],
        default: 'file',
      },
      ignore: {
        alias: 'i',
        describe: 'Ignore folders/files',
        type: 'array',
        default: [],
      }
    },
    handler: (_) => {},
  };

  const result = yargs(hideBin(process.argv))
    .scriptName('spinne')
    .command(scanCommand)
    .help()
    .parseSync();

  const options = result as unknown as Return['options'];
  const command = result._[0] as 'scan';

  return { options, command };
};
