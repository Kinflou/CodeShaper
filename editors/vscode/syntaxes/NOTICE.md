# Third-party grammar

`kdl.tmLanguage.json` is the KDL 1.0 grammar from
[kdl-org/vscode-kdl](https://github.com/kdl-org/vscode-kdl) at tag `v1.3.1`, licensed under the
Apache License 2.0 (see `LICENSE.kdl`).

Changes: `name` is `KDL (CodeShaper)` and `scopeName` is `source.kdl.codeshaper`, so it does not
clash with the official KDL extension, which only supports KDL 2.

`expressions.tmLanguage.json` is CodeShaper's own injection grammar for `#[…]`, `$[…]` and `@[…]`
inside strings.
