# collclean

[![crates.io](https://img.shields.io/crates/v/collclean.svg)](https://crates.io/crates/collclean)
![actively developed](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![dependency status](https://deps.rs/crate/collclean/0.2.0/status.svg)](https://deps.rs/crate/collclean/0.2.0)
![License: MIT/Apache-2.0](https://img.shields.io/crates/l/collclean.svg)

Installation:

```bash
cargo install collclean
```

Example call

```bash
collclean file.tex mycomm1 mycomm2 ...
```

`file.tex` (if it does not compile, collclean will remove nothing)

```tex
\mycomm1{I wrote that!}
\mycomm2{I wrote that! \mycomm2{lalalala} y\{y{y}y{} }
```

`file.tex` afterwards

```tex
I wrote that!
I wrote that! lalalala y\{y{y}y{} 
```

The command definitions (e.g. via `\newcommand`) will not get removed.

Output to new file

```bash
collclean file.tex mycomm1 -o new_file.tex
```
