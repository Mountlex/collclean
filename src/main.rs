use anyhow::Result;
use clap::{arg, Command};

fn main() -> Result<()> {
    let matches = Command::new("collclean")
        .version("0.1")
        .author("Alexander Lindermayr <alexander.lindermayr97@gmail.com>")
        .about("Clean LaTeX files after a collaboration.")
        .arg(arg!(<FILE>))
        .arg(arg!([COMMANDS]).required(true).multiple_values(true).takes_value(true).allow_invalid_utf8(true))
        .get_matches();

    let path = matches.value_of_os("FILE").map(std::path::PathBuf::from);
    let commands = matches.values_of("COMMANDS").unwrap().collect();
    if let Some(path) = path {
        if path.exists() {
            let mut text = std::fs::read_to_string(&path)?;
            clean(&mut text, commands);
            std::fs::write(path, text)?;
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

fn clean(text: &mut String, commands: Vec<&str>) {
    let mut patterns: Vec<Pattern> = commands
        .into_iter()
        .map(|comm| {
            Pattern::new(
                &format!("{}{{", comm),
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

    for (i, c) in text.char_indices() {
        'patterns: for p in patterns.iter_mut() {
            if let Some(s) = p.next(i, c) {
                match (&p.processing, &p.count) {
                    (Process::Keep, BracketCount::CountUp) => depth += 1,
                    (Process::Keep, BracketCount::CountDown) => depth -= 1,
                    (Process::Delete, BracketCount::CountUp) => {
                        deleted_depths.push(depth);
                        depth += 1;
                        let deletion = Deletion::range(s, s + p.len() - 1);
                        deletions.push(deletion);
                    }
                    (Process::DeleteOnDepth, BracketCount::CountDown) => {
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

    let mut deleted: usize = 0;
    for del in deletions {
        //println!("del from {} to {}", del.start, del.end);
        let start = del.start - deleted;
        let end = del.end - deleted;
        text.replace_range(start..=end, "");
        deleted += del.len();
    }
}

#[cfg(test)]
mod test_clean {
    use super::*;

    #[test]
    fn test_no_clean() {
        let mut text1 = String::from("\\{  \\}");
        let mut text2 = String::from("{  }");
        clean(&mut text1, vec![]);
        clean(&mut text2, vec![]);
        assert_eq!(text1, "\\{  \\}");
        assert_eq!(text2, "{  }");
    }

    #[test]
    fn test_simple_clean() {
        let mut text = String::from("\\anew{ab}");
        clean(&mut text, vec!["\\anew"]);
        assert_eq!(text, "ab");
    }

    #[test]
    fn test_clean() {
        let mut text = String::from("\\anew{ a{v}b \\{ }");
        clean(&mut text, vec!["\\anew"]);
        assert_eq!(text, " a{v}b \\{ ");
    }

    #[test]
    fn test_clean_2() {
        let mut text = String::from("\\anew{\\nnew{ a{v}b} \\{ }");
        clean(&mut text, vec!["\\anew", "\\nnew"]);
        assert_eq!(text, " a{v}b \\{ ");
    }
}
