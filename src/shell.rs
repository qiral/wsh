use crate::config::Config;
use crate::utils::Utils;
use anyhow::{Result, anyhow};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::collections::VecDeque;
use std::io::{Write, stdout};
use std::process::{Command, Stdio};

pub struct Shell {
    config: Config,
    history: VecDeque<String>,
    current_input: String,
    cursor_pos: usize,
    history_index: Option<usize>,
}

impl Shell {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            history: VecDeque::new(),
            current_input: String::new(),
            cursor_pos: 0,
            history_index: None,
        })
    }

    pub fn execute_command(&mut self, command: &str) -> Result<()> {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        // Add to history
        self.add_to_history(trimmed.to_string());

        let tokens = Utils::parse_command(trimmed);
        if tokens.is_empty() {
            return Ok(());
        }

        let command_name = &tokens[0];
        let args = &tokens[1..];

        // Check for aliases
        if let Some(alias_command) = self.config.aliases.get(command_name).cloned() {
            return self.execute_command(&alias_command);
        }

        // Handle built-in commands
        if Utils::is_builtin(command_name) {
            self.execute_builtin(command_name, args)
        } else {
            self.execute_external(command_name, args)
        }
    }

    pub fn run_interactive(&mut self) -> Result<()> {
        println!("Welcome to WSH - A modern shell written in Rust!");
        println!("Type 'help' for available commands or 'exit' to quit.");

        terminal::enable_raw_mode()?;

        loop {
            self.display_prompt()?;

            match self.read_input()? {
                InputResult::Command(cmd) => {
                    println!(); // New line after input
                    if let Err(e) = self.execute_command(&cmd) {
                        self.print_error(&format!("Error: {}", e))?;
                    }
                    self.reset_input();
                }
                InputResult::Exit => break,
                InputResult::Continue => continue,
            }
        }

        terminal::disable_raw_mode()?;
        println!("\nGoodbye!");
        Ok(())
    }

    fn add_to_history(&mut self, command: String) {
        // Don't add duplicate consecutive commands
        if self.history.back() != Some(&command) {
            self.history.push_back(command);

            // Limit history size
            while self.history.len() > self.config.history_size {
                self.history.pop_front();
            }
        }
    }

    fn execute_builtin(&mut self, command: &str, args: &[String]) -> Result<()> {
        match command {
            "cd" => {
                let path = args.get(0).map(String::as_str).unwrap_or("");
                Utils::change_directory(path)?;
                Ok(())
            }
            "pwd" => {
                println!("{}", Utils::get_current_dir()?);
                Ok(())
            }
            "exit" => std::process::exit(0),
            "help" => {
                self.show_help();
                Ok(())
            }
            "history" => {
                self.show_history();
                Ok(())
            }
            "alias" => {
                if args.len() == 2 {
                    self.config.aliases.insert(args[0].clone(), args[1].clone());
                    println!("Alias '{}' -> '{}' added", args[0], args[1]);
                } else {
                    for (alias, command) in &self.config.aliases {
                        println!("{} -> {}", alias, command);
                    }
                }
                Ok(())
            }
            _ => Err(anyhow!("Unknown built-in command: {}", command)),
        }
    }

    fn execute_external(&self, command: &str, args: &[String]) -> Result<()> {
        let output = Command::new(command)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();

        match output {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!("Failed to execute '{}': {}", command, e)),
        }
    }

    fn display_prompt(&self) -> Result<()> {
        let prompt = Utils::format_prompt(&self.config.prompt);

        if self.config.enable_colors {
            execute!(
                stdout(),
                SetForegroundColor(Color::Green),
                Print(&prompt),
                ResetColor,
                Print(&self.current_input)
            )?;
        } else {
            print!("{}{}", prompt, self.current_input);
        }

        // Position cursor
        if self.cursor_pos < self.current_input.len() {
            let remaining = self.current_input.len() - self.cursor_pos;
            execute!(stdout(), cursor::MoveLeft(remaining as u16))?;
        }

        stdout().flush()?;
        Ok(())
    }

    fn read_input(&mut self) -> Result<InputResult> {
        loop {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match (code, modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        return Ok(InputResult::Exit);
                    }
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        if self.current_input.is_empty() {
                            return Ok(InputResult::Exit);
                        }
                    }
                    (KeyCode::Enter, _) => {
                        let command = self.current_input.clone();
                        return Ok(InputResult::Command(command));
                    }
                    (KeyCode::Backspace, _) => {
                        if self.cursor_pos > 0 {
                            self.current_input.remove(self.cursor_pos - 1);
                            self.cursor_pos -= 1;
                            self.redraw_line()?;
                        }
                    }
                    (KeyCode::Delete, _) => {
                        if self.cursor_pos < self.current_input.len() {
                            self.current_input.remove(self.cursor_pos);
                            self.redraw_line()?;
                        }
                    }
                    (KeyCode::Left, _) => {
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            execute!(stdout(), cursor::MoveLeft(1))?;
                        }
                    }
                    (KeyCode::Right, _) => {
                        if self.cursor_pos < self.current_input.len() {
                            self.cursor_pos += 1;
                            execute!(stdout(), cursor::MoveRight(1))?;
                        }
                    }
                    (KeyCode::Up, _) => {
                        self.navigate_history(true)?;
                    }
                    (KeyCode::Down, _) => {
                        self.navigate_history(false)?;
                    }
                    (KeyCode::Home, _) => {
                        execute!(stdout(), cursor::MoveLeft(self.cursor_pos as u16))?;
                        self.cursor_pos = 0;
                    }
                    (KeyCode::End, _) => {
                        let move_right = self.current_input.len() - self.cursor_pos;
                        execute!(stdout(), cursor::MoveRight(move_right as u16))?;
                        self.cursor_pos = self.current_input.len();
                    }
                    (KeyCode::Char(c), _) => {
                        self.current_input.insert(self.cursor_pos, c);
                        self.cursor_pos += 1;
                        self.redraw_line()?;
                    }
                    _ => {}
                }
            }
        }
    }

    fn navigate_history(&mut self, up: bool) -> Result<()> {
        if self.history.is_empty() {
            return Ok(());
        }

        let new_index = match self.history_index {
            None if up => Some(self.history.len() - 1),
            None => return Ok(()),
            Some(i) if up && i > 0 => Some(i - 1),
            Some(i) if !up && i < self.history.len() - 1 => Some(i + 1),
            Some(_) if !up => {
                self.history_index = None;
                self.current_input.clear();
                self.cursor_pos = 0;
                self.redraw_line()?;
                return Ok(());
            }
            _ => return Ok(()),
        };

        self.history_index = new_index;
        if let Some(index) = new_index {
            self.current_input = self.history[index].clone();
            self.cursor_pos = self.current_input.len();
            self.redraw_line()?;
        }

        Ok(())
    }

    fn redraw_line(&self) -> Result<()> {
        execute!(
            stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::FromCursorDown)
        )?;
        self.display_prompt()?;
        Ok(())
    }

    fn reset_input(&mut self) {
        self.current_input.clear();
        self.cursor_pos = 0;
        self.history_index = None;
    }

    fn show_help(&self) {
        println!("WSH - Built-in Commands:");
        println!("  cd [path]     - Change directory");
        println!("  pwd           - Print working directory");
        println!("  history       - Show command history");
        println!("  alias [name] [cmd] - Create or show aliases");
        println!("  help          - Show this help message");
        println!("  exit          - Exit the shell");
        println!("\nKeyboard shortcuts:");
        println!("  Ctrl+C / Ctrl+D - Exit");
        println!("  Up/Down arrows  - Navigate history");
        println!("  Left/Right      - Move cursor");
        println!("  Home/End        - Jump to line start/end");
    }

    fn show_history(&self) {
        if self.history.is_empty() {
            println!("No history available");
            return;
        }

        for (i, cmd) in self.history.iter().enumerate() {
            println!("{:4}: {}", i + 1, cmd);
        }
    }

    fn print_error(&self, message: &str) -> Result<()> {
        if self.config.enable_colors {
            execute!(
                stdout(),
                SetForegroundColor(Color::Red),
                Print(message),
                ResetColor,
                Print("\n")
            )?;
        } else {
            println!("{}", message);
        }
        Ok(())
    }
}

enum InputResult {
    Command(String),
    Exit,
   Continue,
}
