use crate::completion::Completion;
use crate::config::Config;
use crate::ui::UI;
use crate::utils::Utils;
use anyhow::{Result, anyhow};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::Print,
    terminal,
};
use std::collections::VecDeque;
use std::io::stdout;
use std::process::Command;

pub struct Shell {
    config: Config,
    history: VecDeque<String>,
    current_input: String,
    cursor_pos: usize,
    history_index: Option<usize>,
    completion: Completion,
}

impl Shell {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            history: VecDeque::new(),
            current_input: String::new(),
            cursor_pos: 0,
            history_index: None,
            completion: Completion::new(),
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
        UI::display_welcome()?;

        terminal::enable_raw_mode()?;

        loop {
            UI::display_prompt(&self.config, &self.current_input, self.cursor_pos)?;

            match self.read_input()? {
                InputResult::Command(cmd) => {
                    UI::print_newline()?; // New line after input
                    if let Err(e) = self.execute_command(&cmd) {
                        UI::print_error(&self.config, &format!("Error: {}", e))?;
                    }
                    self.reset_input();
                }
                InputResult::Exit => break,
            }
        }

        terminal::disable_raw_mode()?;
        UI::display_goodbye()?;
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
                let current_dir = Utils::get_current_dir()?;
                execute!(stdout(), Print(&format!("{}\n", current_dir)))?;
                Ok(())
            }
            "exit" => std::process::exit(0),
            "help" => {
                UI::show_help()?;
                Ok(())
            }
            "history" => {
                UI::show_history(&self.history)?;
                Ok(())
            }
            "alias" => {
                if args.len() == 2 {
                    self.config.aliases.insert(args[0].clone(), args[1].clone());
                    execute!(
                        stdout(),
                        Print(&format!("Alias '{}' -> '{}' added\n", args[0], args[1]))
                    )?;
                } else {
                    for (alias, command) in &self.config.aliases {
                        execute!(stdout(), Print(&format!("{} -> {}\n", alias, command)))?;
                    }
                }
                Ok(())
            }
            _ => Err(anyhow!("Unknown built-in command: {}", command)),
        }
    }

    fn execute_external(&self, command: &str, args: &[String]) -> Result<()> {
        // Disable raw mode temporarily for external commands
        terminal::disable_raw_mode()?;

        let result = Command::new(command).args(args).status(); // Use .status() instead of .output()

        // Re-enable raw mode
        terminal::enable_raw_mode()?;

        match result {
            Ok(status) => {
                if status.success() {
                    Ok(())
                } else {
                    Err(anyhow!("Command '{}' exited with non-zero status", command))
                }
            }
            Err(e) => Err(anyhow!("Failed to execute '{}': {}", command, e)),
        }
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
                        self.reset_completion();
                        if self.cursor_pos > 0 {
                            self.current_input.remove(self.cursor_pos - 1);
                            self.cursor_pos -= 1;
                            UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
                        }
                    }
                    (KeyCode::Delete, _) => {
                        self.reset_completion();
                        if self.cursor_pos < self.current_input.len() {
                            self.current_input.remove(self.cursor_pos);
                            UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
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
                    (KeyCode::Tab, _) => {
                        self.handle_tab_completion()?;
                    }
                    (KeyCode::Char(c), _) => {
                        self.reset_completion();
                        self.current_input.insert(self.cursor_pos, c);
                        self.cursor_pos += 1;
                        UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
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
                UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
                return Ok(());
            }
            _ => return Ok(()),
        };

        self.history_index = new_index;
        if let Some(index) = new_index {
            self.current_input = self.history[index].clone();
            self.cursor_pos = self.current_input.len();
            UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
        }

        Ok(())
    }

    fn reset_input(&mut self) {
        self.current_input.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        self.reset_completion();
    }

    fn reset_completion(&mut self) {
        self.completion.reset();
    }

    fn handle_tab_completion(&mut self) -> Result<()> {
        if self.completion.is_empty() {
            self.completion.generate(
                &self.current_input,
                self.cursor_pos,
                &self.config,
                &self.history,
            );
            if self.completion.is_empty() {
                return Ok(());
            }
            self.completion.start(&self.current_input, self.cursor_pos);
            self.completion
                .apply(&mut self.current_input, &mut self.cursor_pos)?;
            UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
        } else {
            self.completion.cycle_next();
            self.completion
                .apply(&mut self.current_input, &mut self.cursor_pos)?;
            UI::redraw_line(&self.config, &self.current_input, self.cursor_pos)?;
        }
        Ok(())
    }

    // completion-specific helper methods removed; logic now in Completion
}

enum InputResult {
    Command(String),
    Exit,
}
