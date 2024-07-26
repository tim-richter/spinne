import type { Command } from "commander";
import { createProgram } from "./utils/cliArgs.js";

export const run = async (program: Command) => {
  await createProgram(program, process.argv);
};
