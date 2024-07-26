import { describe, expect, it } from "vitest";
import { resolve } from "../../utils/path.js";
import { analyzeProjectExports } from "./exports.js";
import { scanAST } from "./index.js";

describe("exports", () => {
  it("should scan exports from a index file correctly", () => {
    const result = analyzeProjectExports(
      [resolve("fixtures/project-exports/simple/src/index.ts")],
      { jsx: 4 },
    );

    expect(result).toMatchInlineSnapshot(`
      [
        "Export: Button (FC)",
      ]
    `);
  });
});

describe("scanAST", () => {
  it("should scan components correctly", () => {
    const code = `
      import { Button } from 'my-library';
  
      export const CustomButton = () => {
        return <Button variant="blue" />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
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
                "column": 22,
                "line": 5,
              },
              "start": {
                "column": 16,
                "line": 5,
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
        ],
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should scan components that have a js value as prop", () => {
    const code = `
      import { Button } from 'my-library';
  
      export const CustomButton = () => {
        const data = {
          test: 'test',
        }
  
        return <Button variant="blue" data={data} />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
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
                "column": 22,
                "line": 9,
              },
              "start": {
                "column": 16,
                "line": 9,
              },
            },
            "name": "Button",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
              {
                "name": "data",
                "value": "(Identifier)",
              },
            ],
            "propsSpread": false,
          },
        ],
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should set propsSpread to true if the component spreads its props", () => {
    const code = `
      import { Button } from 'my-library';
  
      export const CustomButton = () => {
        const data = {
          test: 'test',
        }
  
        return <Button variant="blue" {...data} />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
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
                "column": 22,
                "line": 9,
              },
              "start": {
                "column": 16,
                "line": 9,
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
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should get the correct component when spreading the component name", () => {
    const code = `
      import MyLibrary from 'my-library';
  
      const { Button } = MyLibrary;
  
      export const CustomButton = () => {
        const data = {
          test: 'test',
        }
  
        return <Button variant="blue" {...data} />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
      {
        "components": [
          {
            "location": {
              "end": {
                "column": 22,
                "line": 11,
              },
              "start": {
                "column": 16,
                "line": 11,
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
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should only include jsx component imports in the report", () => {
    const code = `
      import MyLibrary from 'my-library';
      import { bla } from '../bla';
      import Bla from 'bla';
  
      const { Button } = MyLibrary;
  
      export const CustomButton = () => {
        const data = {
          test: 'test',
        }
  
        return <Button variant="blue" {...data} />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
      {
        "components": [
          {
            "location": {
              "end": {
                "column": 22,
                "line": 13,
              },
              "start": {
                "column": 16,
                "line": 13,
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
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should parse literal values from JSXExpressionContainers", () => {
    const code = `
      import { Button } from 'my-library';
  
      export const CustomButton = () => {
        const name = 'world';
  
        return <Button variant="blue"  text={"name"} />;
      } 
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
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
                "column": 22,
                "line": 7,
              },
              "start": {
                "column": 16,
                "line": 7,
              },
            },
            "name": "Button",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
              {
                "name": "text",
                "value": "name",
              },
            ],
            "propsSpread": false,
          },
        ],
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should destructure the name of the component if it was used with dot syntax", () => {
    const code = `
      import MyLibrary from 'my-library';
      import { bla } from '../bla';
      import Bla from 'bla';
  
      export const CustomButton = () => {
        const data = {
          test: 'test',
        }
  
        return <MyLibrary.Button variant="blue" {...data} />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
      {
        "components": [
          {
            "importInfo": {
              "importType": "ImportDefaultSpecifier",
              "local": "MyLibrary",
              "moduleName": "my-library",
            },
            "location": {
              "end": {
                "column": 32,
                "line": 11,
              },
              "start": {
                "column": 16,
                "line": 11,
              },
            },
            "name": "MyLibrary.Button",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
            ],
            "propsSpread": true,
          },
        ],
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should scan jsx member expressions", () => {
    const code = `
      export const CustomButton = ({ as }) => {
        const As = as || 'div'
  
        return <As variant="blue" {...data} />;
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
      {
        "components": [
          {
            "location": {
              "end": {
                "column": 18,
                "line": 5,
              },
              "start": {
                "column": 16,
                "line": 5,
              },
            },
            "name": "As",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
            ],
            "propsSpread": true,
          },
        ],
        "filePath": "src/test/Component.tsx",
      }
    `);
  });

  it("should scan jsx namespaced name", () => {
    const code = `
      import * as mynamespace from 'my-namespace';
  
      export const CustomButton = ({ as }) => {
        return <mynamespace:Menu variant="blue" />
      }
    `;

    const ast = scanAST({ code, filePath: "src/test/Component.tsx" });

    expect(ast).toMatchInlineSnapshot(`
      {
        "components": [
          {
            "importInfo": {
              "importType": "ImportNamespaceSpecifier",
              "local": "mynamespace",
              "moduleName": "my-namespace",
            },
            "location": {
              "end": {
                "column": 32,
                "line": 5,
              },
              "start": {
                "column": 16,
                "line": 5,
              },
            },
            "name": "mynamespace.Menu",
            "props": [
              {
                "name": "variant",
                "value": "blue",
              },
            ],
            "propsSpread": false,
          },
        ],
        "filePath": "src/test/Component.tsx",
      }
    `);
  });
});
