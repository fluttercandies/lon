# LON for VS Code

Syntax highlighting and language configuration for [LON](https://github.com/fluttercandies/lon) (Language Object Notation v1) — a compact, canonical, lossless encoding of the JSON data model.

## Features

- `.lon` file detection
- TextMate highlighting for structural punctuation, quoted and bare strings, numbers, `null`/`true`/`false` literals, object keys, and record-table schema headers
- Bracket matching, auto-closing pairs, surrounding pairs, and indentation rules for `{ }` and `[ ]`

LON has no comment syntax, so no comment keybindings are defined.

## Install

### From source

```sh
git clone https://github.com/fluttercandies/lon
cd lon/editors/vscode
npm install -g @vscode/vsce   # once
vsce package                  # produces lon-<version>.vsix
code --install-extension lon-<version>.vsix
```

Or symlink/copy this directory into `~/.vscode/extensions/` and reload VS Code.

Marketplace publication is planned; install from `.vsix` in the meantime.

## Validation

This extension highlights syntax only — it does not validate documents. For real validation use the repository CLI's `lon check` command, e.g. wired as a VS Code task:

```json
{
  "label": "lon check",
  "type": "shell",
  "command": "lon check ${file}",
  "problemMatcher": []
}
```

## License

[BSD-3-Clause](LICENSE)
