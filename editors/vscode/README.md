# CodeShaper for VS Code

Highlighting, diagnostics, completion and previews for CodeShaper patch files.

Highlighting comes from a bundled KDL 1 grammar (see `syntaxes/NOTICE.md`) plus an injection that
colours `#[…]`, `$[…]` and `@[…]` inside strings. Diagnostics, completion, navigation and hover come
from `codeshaper lsp`.

Each patch gets a **Preview result** lens, a preview button in the editor's title bar, and
*CodeShaper: Preview Patch Result* in the command palette. It runs that patch alone, as currently typed, on the original target files its `file`
pattern matches, and opens a diff of the result. When several files would change, pick one first.
The target is found through the operation configuration next to the shaping project.

## Install

Download the `.vsix` for your platform from the
[releases](https://github.com/Kinflou/CodeShaper/releases), then run *Extensions: Install from VSIX…*
or:

```sh
code --install-extension codeshaper-linux-x64.vsix
```

It includes the `codeshaper` server, so nothing else is needed. The newest ten builds of `dev` are the
pre-releases named `dev-<number>`. Installed this way it doesn't update itself.

`codeshaper.server.path` picks another executable. When it is empty, the included server is used, or
`codeshaper` on your `PATH` if there is none.

## Build

1. Install the CLI so `codeshaper` is on your `PATH`, from the repository root:

   ```sh
   cargo install --path cli
   ```

2. Build the extension:

   ```sh
   npm install
   npm run compile
   ```

3. Run it: open this folder in VS Code and press F5, or start VS Code with
   `code --extensionDevelopmentPath=editors/vscode <workspace>`.

`npm run package` makes a `.vsix`. It includes a server only if there is one in `server/`.

It registers the `kdl` language for `.kdl` files and starts the server when one is opened.
Patches inside a project's `patches/` folder get full validation. `settings.kdl` and configurations
get parse errors, and their paths are suggested and checked.
