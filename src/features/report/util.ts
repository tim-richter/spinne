import type { TSESTree } from "@typescript-eslint/typescript-estree";

export const isJSXOpeningElement = (
  node: any,
): node is TSESTree.JSXOpeningElement => node.type === "JSXOpeningElement";

export const isJSXAttribute = (node: any): node is TSESTree.JSXAttribute =>
  node.type === "JSXAttribute";
