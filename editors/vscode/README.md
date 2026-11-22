# CodeShaper for VS Code

Diagnostics, completion and navigation for CodeShaper patch files, served by `codeshaper lsp`.

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

It activates in workspaces that contain a shaping project (`settings.kdl`) and checks `.kdl` files.
Patches inside a project's `patches/` folder get full validation; `settings.kdl`, `configuration.kdl`
and `project.kdl` get parse errors.
