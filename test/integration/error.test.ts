import { expect, it, vi } from 'vitest';
import { scan } from '../../src/index.js';
import { resolve } from '../../src/utils/path.js';

const cwdNoFiles = resolve('fixtures/no-files');

it('should throw error if no files were found to scan', async () => {
  const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => null);
  await scan({ directory: cwdNoFiles });

  expect(errorSpy).toHaveBeenCalledTimes(1);
  expect(errorSpy.mock.calls[0][0]).toMatchInlineSnapshot(
    `"No files found to scan"`,
  );
});
