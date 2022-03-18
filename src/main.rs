use anyhow::{bail, Result};
use clap::{arg, Command};

fn main() -> Result<()> {
    let matches = Command::new("collclean")
        .version("0.1")
        .author("Alexander Lindermayr <alexander.lindermayr97@gmail.com>")
        .about("Clean LaTeX files after a collaboration.")
        .arg(arg!(<FILE>))
        .arg(
            arg!([COMMANDS])
                .required(true)
                .multiple_values(true)
                .takes_value(true)
                .allow_invalid_utf8(true),
        )
        .arg(arg!(-o - -output[output]).allow_invalid_utf8(true))
        .get_matches();

    let path = matches.value_of_os("FILE").map(std::path::PathBuf::from);
    let commands = matches.values_of("COMMANDS").unwrap().collect();
    if let Some(path) = path {
        if path.exists() {
            let mut text = std::fs::read_to_string(&path)?;
            let num = clean(&mut text, commands)?;
            println!("Removed {} commands!", num / 2);
            if let Some(output) = matches.value_of_os("output").map(std::path::PathBuf::from) {
                std::fs::write(output, text)?;
            } else {
                std::fs::write(path, text)?;
            }
        }
    }
    Ok(())
}

struct Pattern {
    text: Vec<char>,
    start: Option<usize>,
    current: usize,
    count: BracketCount,
    processing: Process,
}

// Todo builder

impl Pattern {
    fn new(text: &str, count: BracketCount, processing: Process) -> Self {
        Self {
            text: text.chars().collect(),
            start: None,
            current: 0,
            count,
            processing,
        }
    }

    fn len(&self) -> usize {
        self.text.len()
    }

    fn next(&mut self, idx: usize, c: char) -> Option<usize> {
        if self.text[self.current] == c {
            if self.start.is_none() {
                self.start = Some(idx);
            }
            self.current += 1;
            if self.current == self.text.len() {
                let tmp = self.start;
                self.start = None;
                self.current = 0;
                tmp
            } else {
                None
            }
        } else {
            self.current = 0;
            self.start = None;
            None
        }
    }
}

enum BracketCount {
    CountUp,
    CountDown,
    NoCount,
}

enum Process {
    Keep,
    Delete,
    DeleteOnDepth,
}

struct Deletion {
    start: usize,
    end: usize,
}

impl Deletion {
    fn range(start: usize, end: usize) -> Self {
        Deletion { start, end }
    }

    fn len(&self) -> usize {
        self.end - self.start + 1
    }
}

fn clean(text: &mut String, commands: Vec<&str>) -> Result<usize> {
    let mut patterns: Vec<Pattern> = commands
        .into_iter()
        .map(|comm| {
            Pattern::new(
                &format!("\\{}{{", comm),
                BracketCount::CountUp,
                Process::Delete,
            )
        })
        .collect();

    patterns.push(Pattern::new("\\{", BracketCount::NoCount, Process::Keep));
    patterns.push(Pattern::new("\\}", BracketCount::NoCount, Process::Keep));
    patterns.push(Pattern::new("{", BracketCount::CountUp, Process::Keep));
    patterns.push(Pattern::new(
        "}",
        BracketCount::CountDown,
        Process::DeleteOnDepth,
    ));

    let mut deletions: Vec<Deletion> = vec![];

    let mut depth: usize = 0;

    let mut deleted_depths: Vec<usize> = vec![];

    let mut commented = false;

    let mut last_c: Option<char> = None;

    for (i, c) in text.char_indices() {
        if c == '\n' {
            commented = false;
        }

        if c == '%' && last_c != Some('\\') {
            commented = true;
        }
        last_c = Some(c);

        if !commented {
            'patterns: for p in patterns.iter_mut() {
                if let Some(s) = p.next(i, c) {
                    match (&p.processing, &p.count) {
                        (Process::Keep, BracketCount::CountUp) => depth += 1,
                        (Process::Keep, BracketCount::CountDown) => {
                            if depth == 0 {
                                bail!("It seems that there is a opening bracket without closing counterpart! Stopping! (no changes made) {}", &text[(i.max(10) - 10)..(i+10).min(text.len() - 1)])
                            }
                            depth -= 1
                        }
                        (Process::Delete, BracketCount::CountUp) => {
                            deleted_depths.push(depth);
                            depth += 1;
                            let deletion = Deletion::range(s, s + p.len() - 1);
                            deletions.push(deletion);
                        }
                        (Process::DeleteOnDepth, BracketCount::CountDown) => {
                            if depth == 0 {
                                bail!("It seems that there is a opening bracket without closing counterpart! Stopping! (no changes made) {}", &text[(i.max(10) - 10)..(i+10).min(text.len() - 1)])
                            }
                            depth -= 1;
                            if let Some(last) = deleted_depths.last() {
                                if *last == depth {
                                    deleted_depths.remove(deleted_depths.len() - 1);
                                    let deletion = Deletion::range(s, s + p.len() - 1);
                                    deletions.push(deletion);
                                }
                            }
                        }
                        (_, _) => {}
                    }

                    break 'patterns;
                }
            }
        }
    }

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
    fn test_clean_2() -> Result<()> {
        let mut text = String::from("\\anew{\\nnew{ a{v}b} \\{ }");
        clean(&mut text, vec!["anew", "nnew"])?;
        assert_eq!(text, " a{v}b \\{ ");
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
    fn test_clean_no_comment() -> Result<()> {
        let mut text = String::from("% \\anew{ } \n");
        clean(&mut text, vec!["anew"])?;
        assert_eq!(text, "% \\anew{ } \n");
        Ok(())
    }
}
