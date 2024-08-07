import { defineConfig } from 'tsup'

export default defineConfig({
  entry: ['src/index.ts', 'src/cli.ts'],
  format: ['esm'],
  bundle: true,
  external: [],
  splitting: false,
  sourcemap: true,
  clean: true,
  dts: true,
})
