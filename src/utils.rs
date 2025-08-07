use anyhow::Result;
use std::path::Path;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Utility functions for the shell
pub struct Utils;

impl Utils {
    /// Expand tilde (~) to home directory
    pub fn expand_path(path: &str) -> String {
        if path.starts_with('~') {
            if let Ok(home) = std::env::var("HOME") {
                path.replacen('~', &home, 1)
            } else {
                path.to_string()
            }
        } else {
            path.to_string()
        }
    }

    /// Parse command line into tokens, handling quotes and escapes
    pub fn parse_command(input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                current_token.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' => escape_next = true,
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                ch if in_quotes && ch == quote_char => {
                    in_quotes = false;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                }
                _ => current_token.push(ch),
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }

    /// Check if a command is a built-in command
    pub fn is_builtin(command: &str) -> bool {
        matches!(
            command,
            "cd" | "pwd" | "exit" | "help" | "alias" | "history"
        )
    }

    /// Get the current working directory as a string
    pub fn get_current_dir() -> Result<String> {
        let current_dir = std::env::current_dir()?;
        Ok(current_dir.display().to_string())
    }

    /// Change directory with proper error handling
    pub fn change_directory(path: &str) -> Result<()> {
        let expanded_path = Self::expand_path(path);
        let target_path = if expanded_path.is_empty() {
            std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
        } else {
            expanded_path
        };

        std::env::set_current_dir(Path::new(&target_path))?;
        Ok(())
    }

    /// Format the prompt with current directory and other info
    pub fn format_prompt(config_prompt: &str) -> String {
        let current_dir = Self::get_current_dir().unwrap_or_else(|_| "unknown".to_string());
        let home = std::env::var("HOME").unwrap_or_default();

        // Replace home directory with ~
        let display_dir = if current_dir.starts_with(&home) {
            current_dir.replacen(&home, "~", 1)
        } else {
            current_dir
        };

        config_prompt.replace("{cwd}", &display_dir)
    }

    /// Check if a file is executable
    #[cfg(unix)]
    pub fn is_executable(path: &Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            let permissions = metadata.permissions();
            permissions.mode() & 0o111 != 0
        } else {
            false
        }
    }
}
