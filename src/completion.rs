use crate::config::Config;
use crate::utils::Utils;
use anyhow::Result;
use crossterm::{execute, style::Print};
use std::collections::HashSet;
use std::io::stdout;
use std::path::Path;

pub struct Completion {
    pub completions: Vec<String>,
    pub completion_index: Option<usize>,
    pub completion_prefix: String,
    pub original_input_before_completion: String,
    pub completion_start_pos: usize,
}

impl Completion {
    pub fn new() -> Self {
        Self {
            completions: Vec::new(),
            completion_index: None,
            completion_prefix: String::new(),
            original_input_before_completion: String::new(),
            completion_start_pos: 0,
        }
    }

    pub fn reset(&mut self) {
        self.completions.clear();
        self.completion_index = None;
        self.completion_prefix.clear();
        self.original_input_before_completion.clear();
        self.completion_start_pos = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.completions.is_empty()
    }

    pub fn generate(&mut self, input: &str, cursor_pos: usize, config: &Config, history: &[String]) {
        let input_before_cursor = &input[..cursor_pos];
        let tokens = Utils::parse_command(input_before_cursor);
        
        if tokens.is_empty() || (tokens.len() == 1 && !input_before_cursor.ends_with(' ')) {
            // Complete command name
            let prefix = tokens.first().map(|s| s.as_str()).unwrap_or("");
            self.completion_prefix = prefix.to_string();
            self.completions = self.get_command_completions(prefix, config, history);
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

    pub fn apply(&mut self, input: &mut String, cursor_pos: &mut usize) -> Result<()> {
        if let Some(index) = self.completion_index {
            if let Some(completion) = self.completions.get(index) {
                // Restore original input and apply the selected completion
                *input = self.original_input_before_completion.clone();
                
                // Replace the prefix with the completion
                let end_pos = self.completion_start_pos + self.completion_prefix.len();
                input.replace_range(self.completion_start_pos..end_pos, completion);
                *cursor_pos = self.completion_start_pos + completion.len();
            }
        }
        Ok(())
    }

    pub fn cycle_next(&mut self) {
        if let Some(current_index) = self.completion_index {
            let next_index = (current_index + 1) % self.completions.len();
            self.completion_index = Some(next_index);
        }
    }

    pub fn start(&mut self, input: &str, cursor_pos: usize) {
        self.original_input_before_completion = input.to_string();
        let prefix_len = self.completion_prefix.len();
        self.completion_start_pos = cursor_pos.saturating_sub(prefix_len);
        self.completion_index = Some(0);
    }

    pub fn should_show_info(&self) -> bool {
        self.completions.len() > 1
    }

    pub fn show_info(&self) -> Result<()> {
        if self.completions.len() <= 1 {
            return Ok(());
        }

        execute!(
            stdout(),
            Print(format!(
                "\nCompletions ({}/{}):\n",
                self.completion_index.map(|i| i + 1).unwrap_or(0),
                self.completions.len()
            ))
        )?;
        
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
            execute!(stdout(), Print(format!("  {}{}\n", marker, completion)))?;
        }

        if self.completions.len() > max_display {
            execute!(stdout(), Print(format!("  ... ({} more)\n", self.completions.len() - max_display)))?;
        }

        Ok(())
    }

    fn get_command_completions(&self, prefix: &str, config: &Config, history: &[String]) -> Vec<String> {
        let mut completions = Vec::new();

        // Built-in commands
        let builtins = ["cd", "pwd", "exit", "help", "alias", "history"];
        for builtin in &builtins {
            if builtin.starts_with(prefix) {
                completions.push(builtin.to_string());
            }
        }

        // Aliases
        for alias in config.aliases.keys() {
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
        for cmd in history {
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
}
