import type { TSESTree } from '@typescript-eslint/typescript-estree';

export const isJSXOpeningElement = (
  node: TSESTree.Node,
): node is TSESTree.JSXOpeningElement => node.type === 'JSXOpeningElement';

export const isJSXAttribute = (
  node: TSESTree.Node,
): node is TSESTree.JSXAttribute => node.type === 'JSXAttribute';
