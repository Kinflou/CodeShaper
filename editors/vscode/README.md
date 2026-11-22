# CodeShaper for VS Code

Highlighting, diagnostics and completion for CodeShaper patch files.

Highlighting comes from a bundled KDL 1 grammar (see `syntaxes/NOTICE.md`) plus an injection that
colours `#[…]`, `$[…]` and `@[…]` inside strings. Diagnostics and completion come from
`codeshaper lsp`.

## Setup

1. Install the CLI so `codeshaper` is on your `PATH`, from the repository root:

   ```sh
   cargo install --path cli
   ```

   Or set `codeshaper.server.path` to the executable.

2. Build the extension:

   ```sh
   npm install
   npm run compile
   ```

3. Run it: open this folder in VS Code and press F5, or start VS Code with
   `code --extensionDevelopmentPath=editors/vscode <workspace>`.

It registers the `kdl` language for `.kdl` files and starts the server when one is opened.
Patches inside a project's `patches/` folder get full validation; `settings.kdl`, `configuration.kdl`
and `project.kdl` get parse errors.
