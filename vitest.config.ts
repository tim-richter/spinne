import { type UserConfig, defineConfig } from "vite";
import type { InlineConfig } from "vitest";

interface VitestConfigExport extends UserConfig {
  test: InlineConfig;
}

export default defineConfig({
  test: {
    setupFiles: ["test/setupFiles.ts"],
    coverage: {
      exclude: ["fixtures/**", "bin/**", "src/mocks/**"],
    },
  },
} satisfies VitestConfigExport);
