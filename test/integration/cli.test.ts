import { expect, it } from "vitest";
import { join } from "../../src/utils/path";
import { execa } from 'execa'

const cli = join(__dirname, '../../bin/spinne.js')

it('should exit gracefully if no arg was given', async () => {
  const { stderr, exitCode, stdout } = await execa({ reject: false})`node ${cli}`

  expect(exitCode).toEqual(0)
  expect(stderr).toMatchInlineSnapshot(`""`)
  expect(stdout).toMatchInlineSnapshot(`""`)
})

it('should output help if help is cli arg', async () => {
  const { stdout } = await execa('node', [cli, 'help'])

  expect(stdout).toMatchInlineSnapshot(`
    "spinne [command]

    Commands:
      spinne scan  Scan a directory for components

    Options:
      --version  Show version number                                       [boolean]
      --help     Show help                                                 [boolean]"
  `)
})

it('should scan from current directory', async () => {
  const { stdout, stderr } = await execa`node ${cli} scan`

  expect(stderr).toMatchInlineSnapshot(`""`)
  expect(stdout).toMatchInlineSnapshot(`"INFO: Found 3 files"`)
})

it('should output directly to console if output=console was used', async () => {
  const { stdout } = await execa`node ${cli} scan -o console`

  expect(stdout).toMatchInlineSnapshot(`"[{"components":[{"name":"Button","importInfo":{"imported":"Button","local":"Button","moduleName":"my-library","importType":"ImportSpecifier"},"props":[{"name":"variant","value":"blue"}],"propsSpread":false,"location":{"start":{"line":6,"column":7},"end":{"line":6,"column":13}}},{"name":"Button","importInfo":{"imported":"Button","local":"Button","moduleName":"my-library","importType":"ImportSpecifier"},"props":[{"name":"variant","value":"blue"}],"propsSpread":true,"location":{"start":{"line":7,"column":7},"end":{"line":7,"column":13}}}],"filePath":"fixtures/simple/src/Button.tsx"}]"`)
})
