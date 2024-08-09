import ts, { type CompilerHost } from 'typescript';

export function analyzeProjectExports(
  rootFiles: string[],
  options: ts.CompilerOptions,
  host?: CompilerHost,
): string[] {
  const report: string[] = [];
  const program = ts.createProgram(rootFiles, options, host);
  const checker = program.getTypeChecker();

  for (const sourceFile of program.getSourceFiles()) {
    // Ignore declaration files and node_modules by default
    if (
      sourceFile.isDeclarationFile ||
      /node_modules/.test(sourceFile.fileName)
    ) {
      continue;
    }

    ts.forEachChild(sourceFile, (node) => {
      if (ts.isExportAssignment(node)) {
        // Handle default exports
        report.push('Default export:', node.expression.getText());
      } else if (
        ts.isExportDeclaration(node) &&
        node.exportClause &&
        ts.isNamedExports(node.exportClause)
      ) {
        // Handle named exports
        for (const spec of node.exportClause.elements) {
          const type = checker.getTypeAtLocation(spec.name);
          const typeString = checker.typeToString(type);
          report.push(`Export: ${spec.name.text} (${typeString})`);
        }
      }
    });
  }

  return report;
}
