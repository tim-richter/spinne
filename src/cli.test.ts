import { Command } from "commander";
import { expect, it, vi } from "vitest";
import { run } from "./cli.js";

it("should output help if no matched commands were given", async () => {
  const mockConsole = vi.fn();

  const program = new Command();
  program.configureOutput({
    writeOut: mockConsole,
    writeErr: mockConsole,
  });

  await run(program);

  expect(mockConsole.mock.calls).toMatchInlineSnapshot(`
    [
      [
        "Usage: snoop [options] [command]

    Snoop scans your code for component usage and creates a report.

    Options:
      -V, --version   output the version number
      -h, --help      display help for command

    Commands:
      scan [options]
      help [command]  display help for command
    ",
      ],
      [
        "Usage: snoop [options] [command]

    Snoop scans your code for component usage and creates a report.

    Options:
      -V, --version   output the version number
      -h, --help      display help for command

    Commands:
      scan [options]
      help [command]  display help for command
    ",
      ],
    ]
  `);
});

it("should output help if 'help' command was used", async () => {
  process.argv = ["node", "snoop", "-V"];
  const mockConsole = vi.fn();

  const program = new Command();
  program.configureOutput({
    writeOut: mockConsole,
    writeErr: mockConsole,
  });

  await run(program);

  expect(mockConsole.mock.calls).toMatchInlineSnapshot(`
    [
      [
        "0.0.1
    ",
      ],
      [
        "Usage: snoop [options] [command]

    Snoop scans your code for component usage and creates a report.

    Options:
      -V, --version   output the version number
      -h, --help      display help for command

    Commands:
      scan [options]
      help [command]  display help for command
    ",
      ],
      [
        "Usage: snoop [options] [command]

    Snoop scans your code for component usage and creates a report.

    Options:
      -V, --version   output the version number
      -h, --help      display help for command

    Commands:
      scan [options]
      help [command]  display help for command
    ",
      ],
    ]
  `);
});
