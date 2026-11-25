# CodeShaperExamples
 Example and Tests projects for CodeShaper

| Example | Shows |
| --- | --- |
| [CPP14/Function Name Correction](CPP14/Function%20Name%20Correction) | Builders, collecting, switches and reference locations of replacers |
| [CPP14/Enum Names](CPP14/Enum%20Names) | Constants, variables and collecting from a reference location |
| [Godot/Godot 3 to 4 Migration](Godot/Godot%203%20to%204%20Migration) | Several languages in one project, queries and several patches in a file |
| [Godot/Log Messages](Godot/Log%20Messages) | A variable used by several patches and a reference location in another part of the file |

Each has a `configuration.kdl` to run, the `target` it changes and the `expected_result`. A run writes to `target` and
keeps the originals in `target_backup`, `--dry-run` only shows the changes:

```sh
codeshaper run --dry-run "CPP14/Enum Names/configuration.kdl"
codeshaper run --dry-run --set type=std::string_view "CPP14/Enum Names/configuration.kdl"
```
