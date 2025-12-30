# collclean

[![Crates.io](https://img.shields.io/crates/v/collclean?style=flat-square)](https://crates.io/crates/collclean)
[![Downloads](https://img.shields.io/crates/d/collclean?style=flat-square)](https://crates.io/crates/collclean)
[![License](https://img.shields.io/crates/l/collclean?style=flat-square)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/Mountlex/collclean/ci.yml?style=flat-square&label=CI)](https://github.com/Mountlex/collclean/actions)

A command-line tool to clean up LaTeX files after collaborative editing. Removes custom collaboration markup commands (like `\alice{...}` and `\bob{...}`) while preserving the content inside.

## Usage

Consider a _valid_ LaTeX file `paper.tex`, where several parts are marked by collaborators using `\alice` and `\bob`:

```tex
Lorem ipsum \alice{dolor sit amet, consetetur \b \bob {sadipscing} elitr, sed diam nonumy eirmod tempor
invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua.} At vero eos et accusam
et justo duo dolores et ea rebum.
\[
    A = \min \{ B, \bob{C \} }
\]
% \alice{Lorem ipsum dolor sit amet
Stet clita kasd gubergren, \alice{no} sea takimata sanctus est Lorem ipsum dolor sit amet.
Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor
invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam
et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus
est Lorem {ipsum dolor sit amet.}
```

To remove these commands and the corresponding brackets, run

```bash
collclean paper.tex alice bob
```

 The file `paper.tex` will then look like this:

```tex
Lorem ipsum dolor sit amet, consetetur \b sadipscing elitr, sed diam nonumy eirmod tempor
invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam
et justo duo dolores et ea rebum.
\[
    A = \min \{ B, C \}
\]
% \alice{Lorem ipsum dolor sit amet
Stet clita kasd gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet.
Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor
invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam
et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus
est Lorem {ipsum dolor sit amet.}
```

### Options

| Option | Description |
|--------|-------------|
| `-o <file>` | Output to a different file (input file stays untouched) |
| `--dry` | Dry run: preview changes without modifying files |
| `--from <line>` | Start line for partial cleaning (1-indexed, inclusive) |
| `--to <line>` | End line for partial cleaning (1-indexed, inclusive) |

### Notes

* Command definitions (e.g., via `\newcommand`) are **not** removed
* Commented lines (starting with `%`) are ignored
* Files with unbalanced brackets are rejected with an error (no changes made)
* Supports Unicode content in LaTeX files
* Handles both Unix (`\n`) and Windows (`\r\n`) line endings

## Installation

### Pre-built binaries

Download pre-built binaries from the [GitHub Releases](https://github.com/Mountlex/collclean/releases) page.

### From crates.io

After [installing Rust](https://rustup.rs/), install via cargo:

```bash
cargo install collclean
```

### From source

```bash
git clone https://github.com/Mountlex/collclean
cd collclean
cargo install --path .
```
