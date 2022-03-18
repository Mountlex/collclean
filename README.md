# collclean

Example call

```
collclean file.tex \\mycomm1 \\mycomm2 ...
```

File before (must compile!)

```tex
\mycomm1{I wrote that!}
\mycomm2{I wrote that! \mycomm2{lalalala} y\{y{y}y{} }
```


File afterwards 

```tex
I wrote that!
I wrote that! lalalala y\{y{y}y{} 
```

The command definitions (e.g. via `\newcommand`) will not get removed.
