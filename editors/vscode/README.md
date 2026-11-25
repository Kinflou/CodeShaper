# CodeShaper for VS Code

Highlighting, diagnostics, completion and previews for CodeShaper patch files.

Highlighting comes from a bundled KDL 1 grammar (see `syntaxes/NOTICE.md`) plus an injection that
colours `#[…]`, `$[…]` and `@[…]` inside strings. Diagnostics, completion, navigation and hover come
from `codeshaper lsp`.

Each patch gets a **Preview result** lens, a preview button in the editor's title bar, and
*CodeShaper: Preview Patch Result* in the command palette. It runs that patch alone, as currently typed, on the original target files its `file`
pattern matches, and opens a diff of the result. When several files would change, pick one first.
In a file with several `patch "name" { … }` blocks, each block has its own lenses, and the button and the
command preview the block at the cursor, or ask for one.
Beside each location stands how many places it matches, and beside the fields of each action how often it
matches, builds and is called, updated on open and save (`codeshaper.inlayHints.matches`).
These hints are smaller than the code, `editor.inlayHints.fontSize` under `[kdl]` changes it.
Hovering a name, such as `location` or `collect`, explains it, and so do the suggestions while typing.
Hovering a location, `match`, `from`, `build`, `to` or a `$[…]` shows samples (`codeshaper.hover.samples`), with
links that select each place, match and capture in a read-only copy of its target file. Files listed when
hovering `file`, the references of a location and the definition of `file` open as copies too.
The target is found through the operation configuration next to the shaping project.

**Explore target**, next to *Preview operation*, opens a read-only copy of a target file and shows which
location its code belongs to. From a `patch` block's lens, only that block's files are listed. Hovering a block gives its code, its locations and how many places they match in the file, with
links to the next and previous place in the target, all places and the surrounding block, to select the block and to copy the location. It also lists
the patches that locate that code. With a selection, the hover is about the block around it. Blocks of
several lines get a hint, down to `codeshaper.explore.depth`, and hovering a hint shows the same for its block. The status bar shows the location at the
cursor. Places are those of the saved file, before any patch runs. Close the copy to stop.

| Key | While exploring |
| --- | --- |
| `N` / `P` | Next or previous place of the location |
| `A` | All places |
| `U` / `D` | Select the surrounding block, or go back inside |
| `S` | Highlight the block and the other places of its location |
| `H` | Every hint: tap to show or hide, or hold |
| `I` | Show the hover |
| `C` | Copy the location |
| `E` | Edit the file itself |

Click the status bar item for the list. They can be changed in *Keyboard Shortcuts*.

**Variables**, the CodeShaper icon in the Activity Bar, lists the constants and variables of the workspace's projects
and where each value comes from. Click a variable to set it, for previews, hovers and counts in this workspace. Nothing
is written to the project, and runs don't use it. A preview that needs a missing value offers to set it. Projects that
git ignores are left out until one of their variables is set.

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

`npm test` compiles it and tests the modules that don't need VS Code.

| In `src/` | What it does |
| --- | --- |
| `extension.ts` | Starts everything |
| `server.ts`, `settings.ts` | The language server and the settings |
| `preview/` | Patch and operation previews |
| `explore/` | Explore target, with the popup in `explore/popup/` |

In each folder, `index.ts` is where commands and providers are registered.

`npm run package` makes a `.vsix`. It includes a server only if there is one in `server/`.

It registers the `kdl` language for `.kdl` files and starts the server when one is opened.
Patches inside a project's `patches/` folder get full validation. `project.kdl` and configurations
get parse errors, and their paths are suggested and checked.
