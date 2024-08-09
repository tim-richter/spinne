import {
  type AST_NODE_TYPES,
  type TSESTree,
  parse,
} from '@typescript-eslint/typescript-estree';
import { walk } from 'estree-walker';
import fs from 'fs-extra';

import { join } from '../../utils/path.js';
import { scan } from './scan.js';

import { getLogger } from '../../utils/logger.js';
import { isJSXAttribute, isJSXOpeningElement } from './util.js';

interface ImportInfo {
  imported?: string;
  local: string;
  moduleName: string;
  importType:
    | TSESTree.AST_NODE_TYPES.ImportSpecifier
    | TSESTree.AST_NODE_TYPES.ImportDefaultSpecifier
    | TSESTree.AST_NODE_TYPES.ImportNamespaceSpecifier;
}

interface Component {
  name?: string;
  importInfo?: ImportInfo;
  location: {
    end: {
      column: number;
      line: number;
    };
    start: {
      column: number;
      line: number;
    };
  };
  props: Array<{
    name: string;
    value:
      | AST_NODE_TYPES
      | string
      | number
      | bigint
      | boolean
      | RegExp
      | null
      | undefined;
  }>;
  propsSpread: boolean;
}

interface ScanResult {
  filePath?: string;
  components?: Array<Component>;
}

const parseOptions = {
  loc: true,
  jsx: true,
};

interface ImportsMapValue {
  imported?: string;
  local: string;
  moduleName: string;
  importType:
    | TSESTree.AST_NODE_TYPES.ImportSpecifier
    | TSESTree.AST_NODE_TYPES.ImportDefaultSpecifier
    | TSESTree.AST_NODE_TYPES.ImportNamespaceSpecifier;
}
type ImportsMap = Record<string, ImportsMapValue>;

const getComponentNameFromAST = (
  nameObj: TSESTree.JSXTagNameExpression,
): string => {
  if (nameObj.type === 'JSXMemberExpression') {
    return `${getComponentNameFromAST(
      nameObj.object,
    )}.${getComponentNameFromAST(nameObj.property)}`;
  }

  if (nameObj.type === 'JSXNamespacedName') {
    return `${nameObj.namespace.name}.${nameObj.name.name}`;
  }

  return nameObj.name;
};

function getPropValue(
  node: TSESTree.JSXExpression | TSESTree.Literal | TSESTree.JSXElement | null,
) {
  if (!node) return true;

  if (node.type === 'Literal') {
    return node.value;
  }

  if (node.type === 'JSXExpressionContainer') {
    if (node.expression.type === 'Literal') {
      return node.expression.value;
    }

    return `(${node.expression.type})`;
  }
}

interface GetInstanceInfo {
  node: TSESTree.JSXOpeningElement;
  importInfo?: ImportInfo;
}

function getInstanceInfo({ node, importInfo }: GetInstanceInfo): Component {
  const { attributes } = node;

  const result: Component = {
    ...(importInfo !== undefined && { importInfo }),
    props: [],
    propsSpread: false,
    location: {
      start: node.name.loc.start,
      end: node.name.loc.end,
    },
  };

  for (const attribute of attributes) {
    if (isJSXAttribute(attribute)) {
      const { name, value } = attribute;

      const propName = name.name;
      const propValue = getPropValue(value);

      result.props.push({ name: propName.toString(), value: propValue });
    } else if (attribute.type === 'JSXSpreadAttribute') {
      result.propsSpread = true;
    }
  }

  return result;
}

type ScanArgs = {
  code: string;
  filePath: string;
};

export function scanAST({ code, filePath }: ScanArgs): ScanResult {
  const report: ScanResult = {
    components: [],
  };

  const ast = parse(code, parseOptions);

  const importsMap: ImportsMap = {};

  walk(ast, {
    enter(node: TSESTree.Node) {
      if (node.type === 'ImportDeclaration') {
        const { source, specifiers } = node as TSESTree.ImportDeclaration;

        const moduleName = source.value;

        for (const specifier of specifiers) {
          if (specifier.type === 'ImportSpecifier') {
            const imported = specifier.imported.name;
            const local = specifier.local.name;

            importsMap[local] = {
              ...(imported !== null && { imported }),
              local,
              moduleName,
              importType: specifier.type,
            };
          } else if (specifier.type === 'ImportDefaultSpecifier') {
            const local = specifier.local.name;

            importsMap[local] = {
              local,
              moduleName,
              importType: specifier.type,
            };
          } else {
            const local = specifier.local.name;

            importsMap[local] = {
              local,
              moduleName,
              importType: specifier.type,
            };
          }
        }
      }
    },
    leave(node: TSESTree.Node) {
      if (isJSXOpeningElement(node)) {
        const { name } = node;

        report.filePath = filePath;

        const nameFromAst = getComponentNameFromAST(name);

        const nameParts = nameFromAst.split('.');

        const [firstPart] = nameParts;

        const info = getInstanceInfo({
          node,
          importInfo: importsMap[firstPart],
        });

        report.components?.push({ name: nameFromAst, ...info });
      }
    },
  });

  return report;
}

export const makeReport = async (cwd: string) => {
  const files = scan(cwd);
  getLogger().info(`Found ${files.length} files`);

  const report: Array<ScanResult> = [];

  for (const file of files) {
    const code = fs.readFileSync(join(cwd, file), 'utf8');

    try {
      const scannedCode = scanAST({ code, filePath: file });

      if (scannedCode?.components && scannedCode.components.length > 1)
        report.push(scannedCode);
    } catch (e) {
      console.error(`Error while scanning file ${file}`, e);
    }
  }

  return report;
};
