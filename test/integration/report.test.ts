import fs from "fs-extra";
import { expect, it, vi } from "vitest";
import type { DeepMockProxy } from "vitest-mock-extended";
import { scan } from "../../src/scan.js";
import { resolve } from "../../src/utils/path.js";

const cwdEmpty = resolve("fixtures/empty");
const cwdSimple = resolve("fixtures/simple");

it("should create an empty report if no components were found", async () => {
  vi.spyOn(fs, "writeJSON").mockImplementation(() => Promise.resolve());
  const mockedFs = fs as unknown as DeepMockProxy<typeof fs>;

  await scan({ directory: cwdEmpty });

  expect(mockedFs.writeJSON).toHaveBeenCalledTimes(1);
  expect(mockedFs.writeJSON.mock.calls[0][1]).toMatchInlineSnapshot(`
    [
      {
        "components": [],
      },
    ]
  `);
});

it("should create a basic report", async () => {
  vi.spyOn(fs, "writeJSON").mockImplementation(() => Promise.resolve());
  const mockedFs = fs as unknown as DeepMockProxy<typeof fs>;

  await scan({ directory: cwdSimple });

  expect(mockedFs.writeJSON).toHaveBeenCalledTimes(1);
  expect(mockedFs.writeJSON.mock.calls[0][1]).toMatchInlineSnapshot(`
    [
      {
        "components": [
          {
            "importInfo": {
              "importType": "ImportSpecifier",
              "imported": "Button",
              "local": "Button",
              "moduleName": "my-library",
            },
            "location": {
              "end": {
                "column": 13,
                "line": 6,
              },
              "start": {
                "column": 7,
                "line": 6,
              },
            },
            "name": "Button",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
            ],
            "propsSpread": false,
          },
          {
            "importInfo": {
              "importType": "ImportSpecifier",
              "imported": "Button",
              "local": "Button",
              "moduleName": "my-library",
            },
            "location": {
              "end": {
                "column": 13,
                "line": 7,
              },
              "start": {
                "column": 7,
                "line": 7,
              },
            },
            "name": "Button",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
            ],
            "propsSpread": true,
          },
        ],
        "filePath": "src/Button.tsx",
      },
    ]
  `);
});
