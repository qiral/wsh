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
use std::collections::{VecDeque, HashSet};
use std::io::{Write, stdout};
use std::process::{Command, Stdio};
use std::path::Path;

pub struct Shell {
    config: Config,
    history: VecDeque<String>,
    current_input: String,
    cursor_pos: usize,
    history_index: Option<usize>,
    completions: Vec<String>,
    completion_index: Option<usize>,
    completion_prefix: String,
    original_input_before_completion: String,
    completion_start_pos: usize,
}

impl Shell {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            history: VecDeque::new(),
            current_input: String::new(),
            cursor_pos: 0,
            history_index: None,
            completions: Vec::new(),
            completion_index: None,
            completion_prefix: String::new(),
            original_input_before_completion: String::new(),
            completion_start_pos: 0,
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
                        self.reset_completion();
                        if self.cursor_pos > 0 {
                            self.current_input.remove(self.cursor_pos - 1);
                            self.cursor_pos -= 1;
                            self.redraw_line()?;
                        }
                    }
                    (KeyCode::Delete, _) => {
                        self.reset_completion();
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
                    (KeyCode::Tab, _) => {
                        self.handle_tab_completion()?;
                    }
                    (KeyCode::Char(c), _) => {
                        self.reset_completion();
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
        self.reset_completion();
    }

    fn reset_completion(&mut self) {
        self.completions.clear();
        self.completion_index = None;
        self.completion_prefix.clear();
        self.original_input_before_completion.clear();
        self.completion_start_pos = 0;
    }

    fn handle_tab_completion(&mut self) -> Result<()> {
        if self.completions.is_empty() {
            // First tab - generate completions and save original state
            self.original_input_before_completion = self.current_input.clone();
            self.generate_completions();
            if self.completions.is_empty() {
                return Ok(());
            }
            
            // Calculate where the completion should start
            let prefix_len = self.completion_prefix.len();
            self.completion_start_pos = self.cursor_pos.saturating_sub(prefix_len);
            
            self.completion_index = Some(0);
            self.apply_completion()?;
        } else {
            // Subsequent tabs - cycle through completions
            if let Some(current_index) = self.completion_index {
                let next_index = (current_index + 1) % self.completions.len();
                self.completion_index = Some(next_index);
                self.apply_completion()?;
            }
        }
        Ok(())
    }

    fn generate_completions(&mut self) {
        let input_before_cursor = &self.current_input[..self.cursor_pos];
        let tokens = Utils::parse_command(input_before_cursor);
        
        if tokens.is_empty() || (tokens.len() == 1 && !input_before_cursor.ends_with(' ')) {
            // Complete command name
            let prefix = tokens.first().map(|s| s.as_str()).unwrap_or("");
            self.completion_prefix = prefix.to_string();
            self.completions = self.get_command_completions(prefix);
        } else {
            // Complete file/directory path
            let last_token = if input_before_cursor.ends_with(' ') {
                ""  // If input ends with space, we're starting a new argument
            } else {
                tokens.last().map(|s| s.as_str()).unwrap_or("")
            };
            self.completion_prefix = last_token.to_string();
            self.completions = self.get_path_completions(last_token);
        }
    }

    fn get_command_completions(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();

        // Built-in commands
        let builtins = ["cd", "pwd", "exit", "help", "alias", "history"];
        for builtin in &builtins {
            if builtin.starts_with(prefix) {
                completions.push(builtin.to_string());
            }
        }

        // Aliases
        for alias in self.config.aliases.keys() {
            if alias.starts_with(prefix) {
                completions.push(alias.clone());
            }
        }

        // Commands in PATH
        if let Ok(path_var) = std::env::var("PATH") {
            let mut seen = HashSet::new();
            for path_dir in path_var.split(':') {
                if let Ok(entries) = std::fs::read_dir(path_dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() {
                                if let Some(name) = entry.file_name().to_str() {
                                    if name.starts_with(prefix) && !seen.contains(name) {
                                        // Check if file is executable
                                        if Utils::is_executable(&entry.path()) {
                                            completions.push(name.to_string());
                                            seen.insert(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // History-based completions
        for cmd in &self.history {
            let cmd_tokens = Utils::parse_command(cmd);
            if let Some(first_token) = cmd_tokens.first() {
                if first_token.starts_with(prefix) && !completions.contains(first_token) {
                    completions.push(first_token.clone());
                }
            }
        }

        completions.sort();
        completions.dedup();
        completions
    }

    fn get_path_completions(&self, prefix: &str) -> Vec<String> {
        let mut completions = Vec::new();
        let expanded_prefix = Utils::expand_path(prefix);
        
        let (dir_path, file_prefix) = if expanded_prefix.ends_with('/') {
            (expanded_prefix.as_str(), "")
        } else {
            let path = Path::new(&expanded_prefix);
            if let Some(parent) = path.parent() {
                let parent_str = parent.to_str().unwrap_or(".");
                // If parent is empty string, use current directory
                let dir_path = if parent_str.is_empty() { "." } else { parent_str };
                (dir_path, 
                 path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            } else {
                (".", expanded_prefix.as_str())
            }
        };

        if let Ok(entries) = std::fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Show hidden files only if prefix starts with dot
                    if name.starts_with(file_prefix) && (!name.starts_with('.') || file_prefix.starts_with('.')) {
                        let mut completion = if dir_path == "." {
                            name.to_string()
                        } else if dir_path.ends_with('/') {
                            format!("{}{}", dir_path, name)
                        } else {
                            format!("{}/{}", dir_path, name)
                        };
                        
                        // Add trailing slash for directories
                        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                            completion.push('/');
                        }
                        
                        completions.push(completion);
                    }
                }
            }
        }

        completions.sort();
        completions
    }

    fn apply_completion(&mut self) -> Result<()> {
        if let Some(index) = self.completion_index {
            if let Some(completion) = self.completions.get(index) {
                // Restore original input and apply the selected completion
                self.current_input = self.original_input_before_completion.clone();
                
                // Replace the prefix with the completion
                let end_pos = self.completion_start_pos + self.completion_prefix.len();
                self.current_input.replace_range(self.completion_start_pos..end_pos, completion);
                self.cursor_pos = self.completion_start_pos + completion.len();
                
                self.redraw_line()?;
                
                // Show completion info if there are multiple options
                if self.completions.len() > 1 {
                    println!();
                    self.show_completion_info()?;
                    self.redraw_line()?;
                }
            }
        }
        Ok(())
    }

    fn show_completion_info(&self) -> Result<()> {
        if self.completions.len() <= 1 {
            return Ok(());
        }

        println!("\nCompletions ({}/{}):", 
                self.completion_index.map(|i| i + 1).unwrap_or(0), 
                self.completions.len());
        
        let max_display = 10;
        let start_idx = if self.completions.len() <= max_display {
            0
        } else {
            let current = self.completion_index.unwrap_or(0);
            if current < max_display / 2 {
                0
            } else if current > self.completions.len() - max_display / 2 {
                self.completions.len() - max_display
            } else {
                current - max_display / 2
            }
        };

        for (i, completion) in self.completions.iter()
            .enumerate()
            .skip(start_idx)
            .take(max_display) {
            
            let marker = if Some(i) == self.completion_index { ">" } else { " " };
            println!("  {}{}", marker, completion);
        }

        if self.completions.len() > max_display {
            println!("  ... ({} more)", self.completions.len() - max_display);
        }

        Ok(())
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
        println!("  Tab             - Auto-complete commands and paths");
        println!("\nAutocompletion features:");
        println!("  - Built-in commands");
        println!("  - Executable commands in PATH");
        println!("  - File and directory paths");
        println!("  - Command aliases");
        println!("  - Commands from history");
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
