import { expect, it } from "vitest";
import { join } from "../../src/utils/path";
import {execa} from 'execa'

const cli = join(__dirname, '../../bin/spinne.js')

it('should throw error and output help if no arg was given', async () => {
  const { stderr, exitCode } = await execa({ reject: false})`node ${cli}`

  expect(exitCode).toEqual(1)
  expect(stderr).toMatchInlineSnapshot(`
    "Usage: spinne [options] [command]

    Spins a web of components and analyzes prop usage, adoption and more

    Options:
      -V, --version   output the version number
      -h, --help      display help for command

    Commands:
      scan [options]
      help [command]  display help for command"
  `)
})

it('should output help if help is cli arg', async () => {
  const { stdout } = await execa('node', [cli, 'help'])

  expect(stdout).toMatchInlineSnapshot(`
    "Usage: spinne [options] [command]

    Spins a web of components and analyzes prop usage, adoption and more

    Options:
      -V, --version   output the version number
      -h, --help      display help for command

    Commands:
      scan [options]
      help [command]  display help for command"
  `)
})

it('should scan from current directory', async () => {
  const { stdout } = await execa('node', [cli, 'scan'])

  expect(stdout).toMatchInlineSnapshot(`"INFO: Found 3 files"`)
})
