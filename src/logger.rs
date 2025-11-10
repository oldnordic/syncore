use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write, BufRead};
use std::path::Path;
use anyhow::Result;
use chrono::{Local, NaiveDate};
use crate::taskmaster::Task;

pub trait CogLogger: Send + Sync {
    fn log_step(&self, step: &crate::cognitive_db::Step, task: &Task) -> std::io::Result<()>;
    fn log_summary(&self, task: &Task, reflection: &str) -> std::io::Result<()>;
}

pub struct MarkdownLogger {
    logs_dir: String,
}

impl MarkdownLogger {
    pub fn new<P: AsRef<Path>>(logs_dir: P) -> Self {
        Self {
            logs_dir: logs_dir.as_ref().to_string_lossy().to_string(),
        }
    }

    fn get_file_path(&self) -> String {
        let today = Local::now().date_naive();
        format!("{}/{}.md", self.logs_dir, today)
    }

    fn get_file_path_for_date(&self, date: NaiveDate) -> String {
        format!("{}/{}.md", self.logs_dir, date)
    }

    fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.logs_dir)?;
        Ok(())
    }

    fn open_file(&self) -> std::io::Result<BufWriter<File>> {
        self.ensure_dir()?;
        let path = self.get_file_path();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(BufWriter::new(file))
    }

    fn escape_backticks(content: &str) -> String {
        // Replace triple backticks with quadruple backticks to avoid breaking markdown
        content.replace("```", "````")
    }

    fn format_time(timestamp: i64) -> String {
        let dt = chrono::DateTime::from_timestamp(timestamp, 0)
            .unwrap_or_default();
        dt.format("%H:%M:%S").to_string()
    }

    pub fn tail_logs(&self, n: usize, since_ts: Option<i64>) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let today = Local::now().date_naive();

        // Check today's log file
        let today_path = self.get_file_path_for_date(today);
        if Path::new(&today_path).exists() {
            lines.extend(self.read_lines_from_file(&today_path, since_ts)?);
        }

        // Check yesterday's log file if we need more lines
        if lines.len() < n {
            let yesterday = today - chrono::Duration::days(1);
            let yesterday_path = self.get_file_path_for_date(yesterday);
            if Path::new(&yesterday_path).exists() {
                let yesterday_lines = self.read_lines_from_file(&yesterday_path, since_ts)?;
                lines.extend(yesterday_lines);
            }
        }

        // Return the last n lines
        if lines.len() > n {
            lines = lines.into_iter().rev().take(n).collect();
            lines.reverse();
        }

        Ok(lines)
    }

    fn read_lines_from_file(&self, file_path: &str, since_ts: Option<i64>) -> Result<Vec<String>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut lines = Vec::new();

        for line_result in reader.lines() {
            let line = line_result?;

            // Skip lines before timestamp filter
            if let Some(since) = since_ts {
                if let Some(ts_str) = self.extract_timestamp_from_line(&line) {
                    if let Ok(ts) = ts_str.parse::<i64>() {
                        if ts < since {
                            continue;
                        }
                    }
                }
            }

            lines.push(line.to_string());
        }

        Ok(lines)
    }

    fn extract_timestamp_from_line(&self, line: &str) -> Option<String> {
        // Look for timestamp in meta block
        if let Some(start) = line.find("ts = ") {
            let start = start + 6;
            if let Some(end) = line[start..].find("\n") {
                let ts_str = line[start..start + end].trim();
                return Some(ts_str.to_string());
            }
        }
        None
    }
}

impl CogLogger for MarkdownLogger {
    fn log_step(&self, step: &crate::cognitive_db::Step, task: &Task) -> std::io::Result<()> {
        let time_str = Self::format_time(step.created_at);
        let escaped_content = Self::escape_backticks(&step.content);

        let entry = format!(
            "### [{}] Task {} — {}\n\
             state: {}\n\
             ```toml meta\n\
             task_id = {}\n\
             state = \"{}\"\n\
             ts = {}\n\
             ```\n\
             {}\n\n",
            time_str,
            task.id,
            task.goal,
            step.state,
            task.id,
            step.state,
            step.created_at,
            escaped_content
        );

        let mut writer = self.open_file()?;
        writer.write_all(entry.as_bytes())?;
        writer.flush()?;
        Ok(())
    }

    fn log_summary(&self, task: &Task, reflection: &str) -> std::io::Result<()> {
        let time_str = Self::format_time(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        );
        let escaped_reflection = Self::escape_backticks(reflection);

        let entry = format!(
            "### [{}] Task {} — Summary\n\
             state: Summary\n\
             ```toml meta\n\
             task_id = {}\n\
             state = \"Summary\"\n\
             ts = {}\n\
             ```\n\
             **Reflection:** {}\n\n",
            time_str,
            task.id,
            task.id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            escaped_reflection
        );

        let mut writer = self.open_file()?;
        writer.write_all(entry.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cognitive_db::{CogState, Step};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_logs_create_and_append_ok() {
        let temp_dir = TempDir::new().unwrap();
        let logger = MarkdownLogger::new(temp_dir.path());

        let task = Task {
            id: 1,
            goal: "Test task".to_string(),
            description: "".to_string(),
            status: "open".to_string(),
            priority: 3,
            parent_id: None,
            created_at: 0,
            updated_at: 0,
        };

        let step1 = Step {
            id: 1,
            task_id: Some(1),
            state: "Think".to_string(),
            content: "First thought".to_string(),
            meta_json: "{}".to_string(),
            created_at: 0,
        };

        let step2 = Step {
            id: 2,
            task_id: Some(1),
            state: "Decide".to_string(),
            content: "Decision made".to_string(),
            meta_json: "{}".to_string(),
            created_at: 1,
        };

        logger.log_step(&step1, &task).unwrap();
        logger.log_step(&step2, &task).unwrap();

        let file_path = logger.get_file_path();
        let content = fs::read_to_string(&file_path).unwrap();

        assert!(content.contains("First thought"));
        assert!(content.contains("Decision made"));
        assert!(content.contains("state: Think"));
        assert!(content.contains("state: Decide"));

        // Check order - first thought should come before decision
        let think_pos = content.find("First thought").unwrap();
        let decide_pos = content.find("Decision made").unwrap();
        assert!(think_pos < decide_pos);
    }
}
