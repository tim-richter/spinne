import fs from "fs-extra";
import { makeReport } from "./features/report/index.js";
import { getErrorMessage, reportError } from "./utils/error.js";
import { join } from "./utils/path.js";

export const scan = async (options: { directory: string }) => {
  try {
    const report = await makeReport(options.directory);

    await fs.writeJSON(join(options.directory, "scan-data.json"), report);
  } catch (e) {
    reportError({ message: getErrorMessage(e) });
    process.exit(1);
  }
};
