use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{Context, Result, bail};

#[derive(Clone, Debug, Default)]
pub struct RunOptions {
    pub cwd: Option<PathBuf>,
    pub input: Option<Vec<u8>>,
    pub allow_failure: bool,
}

impl RunOptions {
    pub fn cwd(path: impl Into<PathBuf>) -> Self {
        Self {
            cwd: Some(path.into()),
            ..Self::default()
        }
    }

    pub fn input(mut self, value: impl Into<Vec<u8>>) -> Self {
        self.input = Some(value.into());
        self
    }

    pub fn allow_failure(mut self) -> Self {
        self.allow_failure = true;
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProcessOutput {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait ProcessRunner: Send + Sync {
    fn run(&self, command: &str, args: &[String], options: RunOptions) -> Result<ProcessOutput>;
}

pub type SharedRunner = Arc<dyn ProcessRunner>;

#[derive(Default)]
pub struct SystemRunner;

impl ProcessRunner for SystemRunner {
    fn run(&self, command: &str, args: &[String], options: RunOptions) -> Result<ProcessOutput> {
        let mut process = Command::new(command);
        process
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(cwd) = &options.cwd {
            process.current_dir(cwd);
        }
        if options.input.is_some() {
            process.stdin(Stdio::piped());
        } else {
            process.stdin(Stdio::null());
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Do not flash a second console window for Docker, Podman, or DB clients.
            process.creation_flags(0x0800_0000);
        }

        let mut child = process
            .spawn()
            .with_context(|| format!("could not start {}", format_command(command, args)))?;
        if let Some(input) = options.input
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin
                .write_all(&input)
                .with_context(|| format!("could not write to {command}"))?;
        }
        let raw = child
            .wait_with_output()
            .with_context(|| format!("could not wait for {command}"))?;
        let output = ProcessOutput {
            code: raw.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&raw.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&raw.stderr).into_owned(),
        };
        if output.code != 0 && !options.allow_failure {
            let detail = if output.stderr.trim().is_empty() {
                output.stdout.trim()
            } else {
                output.stderr.trim()
            };
            if detail.is_empty() {
                bail!(
                    "{} failed (exit {})",
                    format_command(command, args),
                    output.code
                );
            }
            bail!(
                "{} failed (exit {})\n{}",
                format_command(command, args),
                output.code,
                detail
            );
        }
        Ok(output)
    }
}

pub fn command_exists(runner: &dyn ProcessRunner, command: &str) -> bool {
    runner
        .run(
            command,
            &["--version".to_owned()],
            RunOptions::default().allow_failure(),
        )
        .is_ok_and(|result| result.code == 0)
}

pub fn format_command(command: &str, args: &[String]) -> String {
    std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(quote_argument)
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_argument(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_./:=@-".contains(character))
    {
        value.to_owned()
    } else {
        format!("{value:?}")
    }
}

pub fn first_line(value: &str) -> &str {
    value.trim().lines().next().unwrap_or("")
}

pub fn compact_output(value: &str) -> String {
    value
        .lines()
        .map(strip_ansi)
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

fn strip_ansi(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for next in characters.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(character);
        }
    }
    result
}

impl fmt::Debug for SystemRunner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SystemRunner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_formatting_quotes_only_when_needed() {
        assert_eq!(
            format_command("docker", &["compose".into(), "two words".into()]),
            "docker compose \"two words\""
        );
    }

    #[test]
    fn output_compaction_removes_colors_and_blank_lines() {
        assert_eq!(
            compact_output("\u{1b}[31mone\u{1b}[0m\n\n two\n"),
            "one / two"
        );
    }
}
