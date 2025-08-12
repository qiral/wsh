use crate::config::Config;
use anyhow::Result;
use crossterm::{
    cursor, execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{Write, stdout};

pub struct UI;

impl UI {
    pub fn display_welcome() -> Result<()> {
        execute!(
            stdout(),
            Print("Welcome to WSH - A modern shell written in Rust!\n")
        )?;
        execute!(
            stdout(),
            Print("Type 'help' for available commands or 'exit' to quit.\n")
        )?;
        Ok(())
    }

    pub fn display_goodbye() -> Result<()> {
        execute!(stdout(), Print("\nGoodbye!\n"))?;
        Ok(())
    }

    pub fn display_prompt(config: &Config, current_input: &str, cursor_pos: usize) -> Result<()> {
        use crate::utils::Utils;
        let prompt = Utils::format_prompt(&config.prompt);

        if config.enable_colors {
            execute!(
                stdout(),
                SetForegroundColor(Color::Green),
                Print(&prompt),
                ResetColor,
                Print(current_input)
            )?;
        } else {
            print!("{}{}", prompt, current_input);
        }

        // Position cursor
        if cursor_pos < current_input.len() {
            let remaining = current_input.len() - cursor_pos;
            execute!(stdout(), cursor::MoveLeft(remaining as u16))?;
        }

        stdout().flush()?;
        Ok(())
    }

    pub fn redraw_line(config: &Config, current_input: &str, cursor_pos: usize) -> Result<()> {
        execute!(
            stdout(),
            Print("\r"), // Move to the start of the line
            terminal::Clear(ClearType::FromCursorDown)
        )?;
        Self::display_prompt(config, current_input, cursor_pos)?;
        Ok(())
    }

    pub fn print_error(config: &Config, message: &str) -> Result<()> {
        execute!(stdout(), Print("Error: "))?;
        if config.enable_colors {
            execute!(
                stdout(),
                SetForegroundColor(Color::Red),
                Print(message),
                ResetColor,
                Print("\r\n")
            )?;
        } else {
            execute!(stdout(), Print(&format!("{}\r\n", message)))?;
        }
        Ok(())
    }

    pub fn show_help() -> Result<()> {
        execute!(stdout(), Print("WSH - Built-in Commands:\n"))?;
        execute!(stdout(), Print("  cd [path]     - Change directory\n"))?;
        execute!(
            stdout(),
            Print("  pwd           - Print working directory\n")
        )?;
        execute!(stdout(), Print("  history       - Show command history\n"))?;
        execute!(
            stdout(),
            Print("  alias [name] [cmd] - Create or show aliases\n")
        )?;
        execute!(
            stdout(),
            Print("  help          - Show this help message\n")
        )?;
        execute!(stdout(), Print("  exit          - Exit the shell\n"))?;
        execute!(stdout(), Print("\nKeyboard shortcuts:\n"))?;
        execute!(stdout(), Print("  Ctrl+C / Ctrl+D - Exit\n"))?;
        execute!(stdout(), Print("  Up/Down arrows  - Navigate history\n"))?;
        execute!(stdout(), Print("  Left/Right      - Move cursor\n"))?;
        execute!(
            stdout(),
            Print("  Home/End        - Jump to line start/end\n")
        )?;
        execute!(
            stdout(),
            Print("  Tab             - Auto-complete commands and paths\n")
        )?;
        execute!(stdout(), Print("\nAutocompletion features:\n"))?;
        execute!(stdout(), Print("  - Built-in commands\n"))?;
        execute!(stdout(), Print("  - Executable commands in PATH\n"))?;
        execute!(stdout(), Print("  - File and directory paths\n"))?;
        execute!(stdout(), Print("  - Command aliases\n"))?;
        execute!(stdout(), Print("  - Commands from history\n"))?;
        Ok(())
    }

    pub fn show_history(history: &std::collections::VecDeque<String>) -> Result<()> {
        if history.is_empty() {
            execute!(stdout(), Print("No history available\n"))?;
            return Ok(());
        }

        for (i, cmd) in history.iter().enumerate() {
            execute!(stdout(), Print(&format!("{:4}: {}\n", i + 1, cmd)))?;
        }
        Ok(())
    }

    pub fn show_completion_info(
        completions: &[String],
        completion_index: Option<usize>,
    ) -> Result<()> {
        if completions.len() <= 1 {
            return Ok(());
        }

        execute!(
            stdout(),
            Print(&format!(
                "\nCompletions ({}/{}):\n",
                completion_index.map(|i| i + 1).unwrap_or(0),
                completions.len()
            ))
        )?;

        let max_display = 10;
        let start_idx = if completions.len() <= max_display {
            0
        } else {
            let current = completion_index.unwrap_or(0);
            if current < max_display / 2 {
                0
            } else if current > completions.len() - max_display / 2 {
                completions.len() - max_display
            } else {
                current - max_display / 2
            }
        };

        for (i, completion) in completions
            .iter()
            .enumerate()
            .skip(start_idx)
            .take(max_display)
        {
            let marker = if Some(i) == completion_index {
                ">"
            } else {
                " "
            };
            execute!(stdout(), Print(&format!("  {}{}\n", marker, completion)))?;
        }

        if completions.len() > max_display {
            execute!(
                stdout(),
                Print(&format!(
                    "  ... ({} more)\n",
                    completions.len() - max_display
                ))
            )?;
        }

        Ok(())
    }

    pub fn print_newline() -> Result<()> {
        execute!(stdout(), Print("\r\n"))?;
        Ok(())
    }
}
