import type { Command } from "commander";
import { scan } from "../scan.js";
import { cwd } from "./path.js";
import { initLogger } from "./logger.js";

export const createProgram = (program: Command, args: string[]) => {
  program
    .name("spinne")
    .description(
      "Spins a web of components and analyzes prop usage, adoption and more",
    )
    .version("0.0.1");

  program
    .command("scan")
    .option(
      "-t, --tsConfig <file>",
      "Typescript configuration path",
      "tsconfig.json",
    )
    .option(
      "-d, --directory <path>",
      "Run process from a different directory",
      cwd(),
    )
    .option(
      "-s, --server <url>",
      "Api Endpoint to send the report to",
      undefined,
    )
    .option('-l, --log-level <level>', 'Set the debug level', 'info')
    .action(async(options) => {
      initLogger(options.logLevel)
      await scan(options)
    });

  const result = program.parseAsync(args);

  return result
};
