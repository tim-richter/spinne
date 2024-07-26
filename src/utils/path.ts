import path from "node:path";

export const join = path.posix.join;

export const toPosix = (value: string) =>
  value.split(path.sep).join(path.posix.sep);

export const cwd = (directory: string = process.cwd()) =>
  path.posix.resolve(directory);

export const resolve = (...paths: string[]) => path.posix.resolve(...paths);
