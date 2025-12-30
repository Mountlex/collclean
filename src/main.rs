use std::fmt::Write;

use anyhow::{bail, Result};
use clap::{arg, ArgAction, Command};
use yansi::Paint;

fn main() -> Result<()> {
    let matches = Command::new("collclean")
        .version("0.4.2")
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

    if let (Some(from), Some(to)) = (from_line, to_line) {
        if from > to {
            bail!("--from ({}) must be less than or equal to --to ({})", from, to);
        }
    }

    let path = matches
        .get_one::<String>("FILE")
        .map(std::path::PathBuf::from);
    let commands = matches
        .get_many::<String>("COMMANDS")
        .expect("no commands")
        .map(|s| s.as_str())
        .collect();

    let path = path.ok_or_else(|| anyhow::anyhow!("No file path provided"))?;
    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

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

fn get_context_around(text: &str, byte_pos: usize, char_count: usize) -> String {
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();
    let char_pos = char_indices
        .iter()
        .position(|(i, _)| *i >= byte_pos)
        .unwrap_or(char_indices.len());

    let start = char_pos.saturating_sub(char_count);
    let end = (char_pos + char_count).min(char_indices.len());

    char_indices[start..end]
        .iter()
        .map(|(_, c)| *c)
        .collect()
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
                    let context = get_context_around(text, i, 10);
                    bail!("It seems that there is a closing bracket without opening counterpart! Stopping! (no changes made) {}", context)
                }
                depth -= 1;
                if let Some(last) = deleted_depths.last() {
                    if *last == depth {
                        deleted_depths.pop();
                        let opening = match deletions.pop() {
                            Some(d) => d,
                            None => {
                                let context = get_context_around(text, i, 10);
                                bail!("It seems that there is a closing bracket without matching opening bracket! Stopping! (no changes made) {}", context)
                            }
                        };
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
            let opening_line = comm.0.line + 1; // Convert to 1-indexed
            let closing_line = comm.1.line + 1; // Convert to 1-indexed
            let from_ok = from.is_none_or(|f| opening_line >= f && closing_line >= f);
            let to_ok = to.is_none_or(|t| opening_line <= t && closing_line <= t);
            from_ok && to_ok
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
                    format!("L{}: ", l + 1).dim()
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

                string.retain(|c| c != '\n' && c != '\r');

                println!("{string}");
            }

            // Handle both Unix (\n) and Windows (\r\n) line endings
            let mut next_start = line_end;
            if text[next_start..].starts_with("\r\n") {
                next_start += 2;
            } else if text[next_start..].starts_with('\n') {
                next_start += 1;
            } else if next_start < text.len() {
                // No newline at end of file, we're done
                next_start = text.len();
            }
            line_start = next_start;
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

fn take_first_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn take_last_chars(s: &str, n: usize) -> String {
    let char_count = s.chars().count();
    s.chars().skip(char_count.saturating_sub(n)).collect()
}

fn add_part(part: &str, string: &mut String, side: Side) -> Result<()> {
    let char_count = part.chars().count();
    if char_count > 10 {
        match side {
            Side::Left => {
                let w = take_last_chars(part, 10);
                string.write_str(&format!("... {w}"))?;
            }
            Side::Center => {
                if char_count <= 30 {
                    string.write_str(part)?;
                } else {
                    let w1 = take_first_chars(part, 10);
                    let w2 = take_last_chars(part, 10);
                    string.write_str(&format!("{w1} ... {w2}"))?;
                }
            }
            Side::Right => {
                let w = take_first_chars(part, 10);
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

    // Tests for UTF-8 handling
    #[test]
    fn test_clean_with_unicode() -> Result<()> {
        let mut text = String::from("\\anew{héllo wörld émoji 🎉}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "héllo wörld émoji 🎉");
        Ok(())
    }

    #[test]
    fn test_clean_unicode_command_content() -> Result<()> {
        let mut text = String::from("Préfix \\anew{中文内容} Süffix");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "Préfix 中文内容 Süffix");
        Ok(())
    }

    #[test]
    fn test_clean_nested_unicode() -> Result<()> {
        let mut text = String::from("\\alice{über \\bob{naïve} café}");
        clean(&mut text, vec!["alice", "bob"])?;
        assert_eq!(text, "über naïve café");
        Ok(())
    }

    // Tests for line range filtering
    fn clean_with_range(
        text: &mut String,
        commands: Vec<&str>,
        from: Option<usize>,
        to: Option<usize>,
    ) -> Result<usize> {
        let deletions = find_deletions(text, commands, from, to)?;
        clean_text(text, deletions)
    }

    #[test]
    fn test_line_range_from() -> Result<()> {
        let mut text = String::from("\\anew{line1}\n\\anew{line2}\n\\anew{line3}");
        clean_with_range(&mut text, vec!["anew"], Some(2), None)?;
        assert_eq!(text, "\\anew{line1}\nline2\nline3");
        Ok(())
    }

    #[test]
    fn test_line_range_to() -> Result<()> {
        let mut text = String::from("\\anew{line1}\n\\anew{line2}\n\\anew{line3}");
        clean_with_range(&mut text, vec!["anew"], None, Some(2))?;
        assert_eq!(text, "line1\nline2\n\\anew{line3}");
        Ok(())
    }

    #[test]
    fn test_line_range_from_to() -> Result<()> {
        let mut text = String::from("\\anew{line1}\n\\anew{line2}\n\\anew{line3}");
        clean_with_range(&mut text, vec!["anew"], Some(2), Some(2))?;
        assert_eq!(text, "\\anew{line1}\nline2\n\\anew{line3}");
        Ok(())
    }

    #[test]
    fn test_line_range_inclusive() -> Result<()> {
        // Verify that --to is inclusive (line 2 should be cleaned)
        let mut text = String::from("\\anew{line1}\n\\anew{line2}\n\\anew{line3}");
        clean_with_range(&mut text, vec!["anew"], Some(1), Some(2))?;
        assert_eq!(text, "line1\nline2\n\\anew{line3}");
        Ok(())
    }

    // Tests for Windows line endings
    #[test]
    fn test_clean_windows_line_endings() -> Result<()> {
        let mut text = String::from("\\anew{line1}\r\n\\anew{line2}\r\n\\anew{line3}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "line1\r\nline2\r\nline3");
        Ok(())
    }

    #[test]
    fn test_clean_mixed_line_endings() -> Result<()> {
        let mut text = String::from("\\anew{line1}\n\\anew{line2}\r\n\\anew{line3}");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "line1\nline2\r\nline3");
        Ok(())
    }

    // Tests for helper functions
    #[test]
    fn test_take_first_chars_ascii() {
        assert_eq!(take_first_chars("hello world", 5), "hello");
        assert_eq!(take_first_chars("hi", 10), "hi");
        assert_eq!(take_first_chars("", 5), "");
    }

    #[test]
    fn test_take_first_chars_unicode() {
        assert_eq!(take_first_chars("héllo wörld", 5), "héllo");
        assert_eq!(take_first_chars("中文内容测试", 3), "中文内");
        assert_eq!(take_first_chars("🎉🎊🎁🎄", 2), "🎉🎊");
    }

    #[test]
    fn test_take_last_chars_ascii() {
        assert_eq!(take_last_chars("hello world", 5), "world");
        assert_eq!(take_last_chars("hi", 10), "hi");
        assert_eq!(take_last_chars("", 5), "");
    }

    #[test]
    fn test_take_last_chars_unicode() {
        assert_eq!(take_last_chars("héllo wörld", 5), "wörld");
        assert_eq!(take_last_chars("中文内容测试", 3), "容测试");
        assert_eq!(take_last_chars("🎉🎊🎁🎄", 2), "🎁🎄");
    }

    #[test]
    fn test_get_context_around_ascii() {
        let text = "0123456789abcdefghij";
        assert_eq!(get_context_around(text, 10, 3), "789abc"); // 3 before 'a', 3 from 'a' onwards
        assert_eq!(get_context_around(text, 0, 3), "012"); // at start, 3 chars from position 0
        assert_eq!(get_context_around(text, 19, 3), "ghij"); // near end, 3 before 'j' + 'j'
    }

    #[test]
    fn test_get_context_around_unicode() {
        let text = "préfix中文süffix";
        // Get context around the Chinese characters
        let ctx = get_context_around(text, 7, 3); // Around '中'
        assert!(ctx.chars().count() <= 6);
        // Should not panic on any position
        for (i, _) in text.char_indices() {
            let _ = get_context_around(text, i, 5);
        }
    }

    // Test error cases produce errors (not panics)
    #[test]
    fn test_unmatched_bracket_error_with_unicode() {
        let text = String::from("héllo } wörld");
        let result = find_deletions(&text, vec!["anew"], None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_bracket_error() {
        let text = String::from("\\anew{ unclosed");
        let result = find_deletions(&text, vec!["anew"], None, None);
        assert!(result.is_err());
    }
}
