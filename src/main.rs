use std::fmt::Write;

use anyhow::{bail, Result};
use clap::{arg, ArgAction, Command};
use yansi::{Paint, Style};

fn main() -> Result<()> {
    let matches = Command::new("collclean")
        .version("0.4.0")
        .author("Alexander Lindermayr <alexander.lindermayr97@gmail.com>")
        .about("Clean LaTeX files after a collaboration.")
        .arg(arg!(<FILE>))
        .arg(arg!([COMMANDS]).required(true).num_args(1..))
        .arg(arg!(-o - -output[output]))
        .arg(arg!(--from[from]).value_parser(clap::value_parser!(usize)))
        .arg(arg!(--to[to]).value_parser(clap::value_parser!(usize)))
        .arg(arg!(--dry[dry]).action(ArgAction::SetTrue))
        .get_matches();

    let dry = matches.get_flag("dry");
    let from_line = matches.get_one::<usize>("from").copied();
    let to_line = matches.get_one::<usize>("to").copied();
    let path = matches
        .get_one::<String>("FILE")
        .map(std::path::PathBuf::from);
    let commands = matches
        .get_many::<String>("COMMANDS")
        .expect("no commands")
        .map(|s| s.as_str())
        .collect();
    if let Some(path) = path {
        if path.exists() {
            let mut text = std::fs::read_to_string(&path)?;
            let deletions = find_deletions(&text, commands, from_line, to_line)?;
            print_deletions(&text, &deletions)?;

            if !dry {
                let num = clean_text(&mut text, deletions)?;
                println!("Removed {} commands!", num / 2);
                if let Some(output) = matches
                    .get_one::<String>("output")
                    .map(std::path::PathBuf::from)
                {
                    std::fs::write(output, text)?;
                } else {
                    std::fs::write(path, text)?;
                }
            }
        }
    }
    Ok(())
}

struct Pattern {
    text: Vec<char>,
    start: Option<usize>,
    current: usize,
    typ: Type,
}

impl Pattern {
    fn new(text: &str, typ: Type) -> Self {
        Self {
            text: text.chars().collect(),
            start: None,
            current: 0,
            typ,
        }
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn reset(&mut self) {
        self.start = None;
        self.current = 0;
    }

    fn next(&mut self, idx: usize, c: char) -> Option<(usize, usize)> {
        if self.current >= self.text.len() && self.typ == Type::Command {
            if c.is_whitespace() && c != '\n' {
                self.current += 1;
                return None;
            } else if c == '{' {
                self.current += 1;
                let tmp = Some((self.start.unwrap(), self.current));
                self.reset();
                return tmp;
            } else {
                self.reset();
            }
        }

        if self.current < self.text.len() && self.text[self.current] != c {
            self.reset();
        }

        if self.current < self.text.len() && self.text[self.current] == c {
            if self.start.is_none() {
                self.start = Some(idx);
            }
            self.current += 1;
            if self.current == self.text.len() && self.typ != Type::Command {
                let tmp = self.start;
                self.reset();
                Some((tmp.unwrap(), self.len()))
            } else {
                None
            }
        } else {
            self.reset();
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
struct Deletion {
    start: usize,
    end: usize,
    line: usize,
}

impl Deletion {
    fn range(start: usize, end: usize, line: usize) -> Self {
        Deletion { start, end, line }
    }

    fn len(&self) -> usize {
        self.end - self.start + 1
    }
}

#[derive(PartialEq)]
enum Type {
    // pattern has an implicit opening bracket
    Command,
    Other,
}

fn find_deletions(
    text: &str,
    commands: Vec<&str>,
    from: Option<usize>,
    to: Option<usize>,
) -> Result<Vec<Deletion>> {
    let mut patterns: Vec<Pattern> = commands
        .into_iter()
        .map(|comm| Pattern::new(&format!("\\{comm}"), Type::Command))
        .collect();
    patterns.push(Pattern::new("\\{", Type::Other));
    patterns.push(Pattern::new("\\}", Type::Other));
    patterns.push(Pattern::new("\\%", Type::Other));

    let mut deletions: Vec<Deletion> = vec![];
    let mut depth: usize = 0;
    let mut deleted_depths: Vec<usize> = vec![];
    let mut commented = false;
    let mut line: usize = 0;
    let mut deleted_commands: Vec<(Deletion, Deletion)> = vec![];

    'chars: for (i, c) in text.char_indices() {
        if c == '\n' {
            line += 1;
        }

        if !commented {
            for p in patterns.iter_mut() {
                if let Some((s, len)) = p.next(i, c) {
                    match p.typ {
                        Type::Command => {
                            deleted_depths.push(depth);
                            depth += 1;
                            let deletion = Deletion::range(s, s + len - 1, line);
                            deletions.push(deletion);
                        }
                        Type::Other => {}
                    }
                    continue 'chars;
                }
            }
        }

        match c {
            '}' if !commented => {
                if depth == 0 {
                    bail!("It seems that there is a closing bracket without opening counterpart! Stopping! (no changes made) {}", &text[(i.max(10) - 10)..(i+10).min(text.len() - 1)])
                }
                depth -= 1;
                if let Some(last) = deleted_depths.last() {
                    if *last == depth {
                        deleted_depths.pop();
                        let opening = deletions.pop().expect("It seems that there is a closing bracket without matching opening bracket! Stopping! (no changes made)");
                        let closing = Deletion::range(i, i, line);

                        deleted_commands.push((opening, closing));
                    }
                }
            }
            '{' if !commented => depth += 1,
            '%' => {
                commented = true;
            }
            '\n' => {
                commented = false;
            }
            _ => {}
        }
    }

    if depth > 0 || !deletions.is_empty() {
        bail!("It seems that there is a opening bracket without closing counterpart! Stopping! (no changes made)")
    }

    let mut final_deletions: Vec<Deletion> = deleted_commands
        .into_iter()
        .filter(|comm| {
            (from.is_none()
                || (comm.0.line + 1 >= from.unwrap() && comm.1.line + 1 >= from.unwrap()))
                && (to.is_none() || (comm.0.line < to.unwrap() && comm.1.line < to.unwrap()))
        })
        .flat_map(|comm| vec![comm.0, comm.1])
        .collect();

    final_deletions.sort();

    Ok(final_deletions)
}

fn print_deletions(text: &str, deletions: &[Deletion]) -> Result<()> {
    if deletions.is_empty() {
        println!("No commands have been found!");
        Ok(())
    } else {
        let mut line_start: usize = 0;
        let mut deletions_iter = deletions.iter();
        let mut current = deletions_iter.next();

        for (l, line) in text.lines().enumerate() {
            if current.is_none() {
                break;
            }

            let line_len = line.len();
            let line_end = line_start + line_len;
            let mut line_deletions = vec![];

            while let Some(del) = current {
                if del.line == l {
                    line_deletions.push(del);
                    current = deletions_iter.next();
                } else {
                    break;
                }
            }

            if !line_deletions.is_empty() {
                let mut string = String::new();
                let line_str = format!(
                    "{}",
                    Style::default().dimmed().paint(format!("L{}: ", l + 1))
                );
                string.write_str(&line_str)?;

                let first_part = &text[line_start..line_deletions.first().unwrap().start];
                add_part(first_part, &mut string, Side::Left)?;
                let first_del = &text
                    [line_deletions.first().unwrap().start..=line_deletions.first().unwrap().end];
                add_del(first_del, &mut string)?;

                for w in line_deletions.windows(2) {
                    let gap = &text[w[0].end + 1..w[1].start];
                    add_part(gap, &mut string, Side::Center)?;
                    let del = &text[w[1].start..=w[1].end];
                    add_del(del, &mut string)?;
                }

                let last_part = &text[line_deletions.last().unwrap().end + 1..line_end];
                add_part(last_part, &mut string, Side::Right)?;

                string.retain(|c| c != '\n');

                println!("{string}");
            }

            line_start = line_end + 1;
        }
        Ok(())
    }
}

fn add_del(part: &str, string: &mut String) -> Result<()> {
    string.write_str(&format!("{}", Paint::red(part).bold()))?;
    Ok(())
}

enum Side {
    Left,
    Center,
    Right,
}

fn add_part(part: &str, string: &mut String, side: Side) -> Result<()> {
    if part.len() > 10 {
        match side {
            Side::Left => {
                let w = &part[part.len() - 10..];
                string.write_str(&format!("... {w}"))?;
            }
            Side::Center => {
                if part.len() <= 30 {
                    string.write_str(part)?;
                } else {
                    let w1 = &part[..10];
                    let w2 = &part[part.len() - 10..];
                    string.write_str(&format!("{w1} ... {w2}"))?;
                }
            }
            Side::Right => {
                let w = &part[..10];
                string.write_str(&format!("{w} ..."))?;
            }
        }
    } else {
        string.write_str(part)?;
    }
    Ok(())
}

fn clean_text(text: &mut String, deletions: Vec<Deletion>) -> Result<usize> {
    let mut deleted: usize = 0;
    let num = deletions.len();
    for del in deletions {
        let start = del.start - deleted;
        let end = del.end - deleted;
        text.replace_range(start..=end, "");
        deleted += del.len();
    }

    Ok(num)
}

#[cfg(test)]
mod test_clean {
    use super::*;

    fn clean(text: &mut String, commands: Vec<&str>) -> Result<usize> {
        let deletions = find_deletions(text, commands, None, None)?;
        clean_text(text, deletions)
    }

    #[test]
    fn test_no_clean() -> Result<()> {
        let mut text1 = String::from("\\{  \\}");
        let mut text2 = String::from("{  }");
        clean(&mut text1, vec![])?;
        clean(&mut text2, vec![])?;
        assert_eq!(text1, "\\{  \\}");
        assert_eq!(text2, "{  }");
        Ok(())
    }

    #[test]
    fn test_simple_clean() -> Result<()> {
        let mut text = String::from("\\anew{ab}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "ab");
        Ok(())
    }

    #[test]
    fn test_clean() -> Result<()> {
        let mut text = String::from("\\anew{ a{v}b \\% \\{ }");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, " a{v}b \\% \\{ ");
        Ok(())
    }

    #[test]
    fn test_clean_double_fake() -> Result<()> {
        let mut text = String::from("\\ane\\anew{ a{v}b \\% \\{ }");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "\\ane a{v}b \\% \\{ ");
        Ok(())
    }

    #[test]
    fn test_clean_double() -> Result<()> {
        let mut text = String::from("\\anew{\\anew{ a{v}b \\% \\{ }}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, " a{v}b \\% \\{ ");
        Ok(())
    }

    #[test]
    fn test_clean_2() -> Result<()> {
        let mut text = String::from("\\anew{\\nnew{ a{v}b} \\{ }");
        clean(&mut text, vec!["anew", "nnew"])?;
        assert_eq!(text, " a{v}b \\{ ");
        Ok(())
    }

    #[test]
    fn test_clean_with_whitespace() -> Result<()> {
        let mut text = String::from("\\anew   { { a{v}b} \\{ }");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, " { a{v}b} \\{ ");
        Ok(())
    }

    #[test]
    fn test_clean_with_whitespace_double() -> Result<()> {
        let mut text = String::from("\\anew  \\anew  { { a{v}b} \\{ }");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "\\anew   { a{v}b} \\{ ");
        Ok(())
    }

    #[test]
    fn test_clean_with_whitespace_2() -> Result<()> {
        let mut text = String::from("\\anew  f { { a{v}b} \\{ } \\anew { f}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "\\anew  f { { a{v}b} \\{ }  f");
        Ok(())
    }

    #[test]
    fn test_clean_fails() -> Result<()> {
        let mut text = String::from("{ } }");
        assert!(clean(&mut text, vec!["anew"]).is_err());
        Ok(())
    }

    #[test]
    fn test_clean_fails_2() -> Result<()> {
        let mut text = String::from("\\anew{ } }");
        assert!(clean(&mut text, vec!["anew"]).is_err());
        Ok(())
    }

    #[test]
    fn test_clean_no_fail() -> Result<()> {
        let mut text = String::from("\\}\\}\\}");
        assert!(clean(&mut text, vec!["anew"]).is_ok());
        Ok(())
    }

    #[test]
    fn test_clean_no_comment_newline() -> Result<()> {
        let mut text = String::from("% % \\anew{ } \n \\anew{}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "% % \\anew{ } \n ");
        Ok(())
    }

    #[test]
    fn test_clean_no_newcommmand() -> Result<()> {
        let mut text = String::from("\\newcommand{\\anew}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "\\newcommand{\\anew}");
        Ok(())
    }
}
