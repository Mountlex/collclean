# collclean

[![crates.io](https://img.shields.io/crates/v/collclean.svg)](https://crates.io/crates/collclean)
![actively developed](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![dependency status](https://deps.rs/crate/collclean/0.3.0/status.svg)](https://deps.rs/crate/collclean/0.3.0)
![License: MIT/Apache-2.0](https://img.shields.io/crates/l/collclean.svg)

## Usage

Consider a valid LaTeX file `paper.tex`, where several parts are marked by collaborators using `\alice` and `\bob`:

```tex
Lorem ipsum \alice{dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor 
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
Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor 
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

### Further notes:

* The command definitions (e.g. via `\newcommand`) will **not** get removed.
* Commented lines are ignored.
* If the original file should stay unchanged, use the option `-o output.tex` to write the cleaned content to the file `output.tex`.

## Installation:

After [installing Rust](https://rustup.rs/), install `collclean` via `cargo`:

```bash
cargo install collclean
```

