import { fdir } from "fdir";

const DEFAULT_GLOBS = ["**/!(*.test|*.spec).@(tsx)"];

export const scan = (crawlFrom: string) => {
  const globs = DEFAULT_GLOBS;

  // eslint-disable-next-line new-cap
  const files = new fdir()
    .glob(...globs)
    .withRelativePaths()
    .exclude((dirName) =>
      dirName.startsWith("node_modules") || dirName.startsWith('.') || dirName.startsWith('dist') 
    )
    .crawl(crawlFrom)
    .sync();

  if (files.length === 0) {
    throw new Error("No files found to scan");
  }

  return files;
};
