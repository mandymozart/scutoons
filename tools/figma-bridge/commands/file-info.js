// file-info – the open file, its current page and how many pages it has (read-only; #93's first module, a template for
// new ones). A module is the body of `async (args, lib) => { … }`, run by the plugin's `run` (README "Command modules").
// Plugin-sandbox syntax: no `?.`, no object spread.
//   node tools/figma-bridge/cli.mjs run file-info
return { file: figma.root.name, page: figma.currentPage.name, pages: figma.root.children.length, core: lib.VERSION };
