use std::sync::mpsc;
use std::time::Instant;

use crate::brain_client;
use crate::commands;
use crate::feed::{FeedItem, FeedSource, Priority, WidgetData};
use crate::ui::BlockColor;

/// Status of a background task.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum TaskStatus {
    Running,
    Completed(String),
    Failed(String),
}

/// A background task being tracked.
pub struct BackgroundTask {
    pub id: u64,
    pub name: String,
    pub status: TaskStatus,
    pub started: Instant,
}

/// Update message from a background task thread.
pub enum TaskUpdate {
    Complete {
        id: u64,
        feed_item: FeedItem,
    },
    Failed {
        id: u64,
        error: String,
    },
}

/// Manages background tasks and their completion.
pub struct TaskManager {
    tasks: Vec<BackgroundTask>,
    next_id: u64,
    task_rx: mpsc::Receiver<TaskUpdate>,
    task_tx: mpsc::Sender<TaskUpdate>,
    max_tasks: usize,
}

impl TaskManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tasks: Vec::new(),
            next_id: 1,
            task_rx: rx,
            task_tx: tx,
            max_tasks: 10,
        }
    }

    /// Spawn a brain query as a background task.
    pub fn spawn_brain_task(&mut self, query: &str) -> Option<u64> {
        if self.active_count() >= self.max_tasks {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;

        let task = BackgroundTask {
            id,
            name: if query.len() > 30 {
                format!("{}...", &query[..27])
            } else {
                query.to_string()
            },
            status: TaskStatus::Running,
            started: Instant::now(),
        };
        self.tasks.push(task);

        let tx = self.task_tx.clone();
        let input = query.to_string();
        std::thread::spawn(move || {
            match brain_client::query_brain(&input) {
                Ok(resp) => {
                    let mut body: Vec<String> = Vec::new();
                    if !resp.text.is_empty() {
                        for line in resp.text.lines() {
                            body.push(line.to_string());
                        }
                    }

                    let mut card = FeedItem::new(
                        FeedSource::Task,
                        Priority::Normal,
                        format!("Task: {}", if input.len() > 40 {
                            format!("{}...", &input[..37])
                        } else {
                            input
                        }),
                    )
                    .with_body(body);

                    if let Some(w) = resp.widgets.first() {
                        let color = match w.widget_type.as_str() {
                            "weather" => BlockColor::Yellow,
                            "system" => BlockColor::Green,
                            "file" => BlockColor::Blue,
                            "table" => BlockColor::Cyan,
                            _ => BlockColor::White,
                        };
                        card = card.with_widget(WidgetData {
                            widget_type: w.widget_type.clone(),
                            title: w.title.clone(),
                            lines: w.lines.clone(),
                            color,
                        });
                    }

                    if resp.latency_ms > 0 {
                        card.body.push(format!("[{:.1}s]", resp.latency_ms as f64 / 1000.0));
                    }

                    let _ = tx.send(TaskUpdate::Complete {
                        id,
                        feed_item: card,
                    });
                }
                Err(e) => {
                    let _ = tx.send(TaskUpdate::Failed {
                        id,
                        error: e,
                    });
                }
            }
        });

        Some(id)
    }

    /// Spawn a shell command as a background task.
    pub fn spawn_shell_task(&mut self, cmd: &str) -> Option<u64> {
        if self.active_count() >= self.max_tasks {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;

        let task = BackgroundTask {
            id,
            name: if cmd.len() > 30 {
                format!("!{}...", &cmd[..27])
            } else {
                format!("!{}", cmd)
            },
            status: TaskStatus::Running,
            started: Instant::now(),
        };
        self.tasks.push(task);

        let tx = self.task_tx.clone();
        let shell_cmd = cmd.to_string();
        std::thread::spawn(move || {
            let result = commands::run_shell(&shell_cmd);
            let card = FeedItem::new(
                FeedSource::Task,
                Priority::Normal,
                format!("Shell: {}", if shell_cmd.len() > 40 {
                    format!("{}...", &shell_cmd[..37])
                } else {
                    shell_cmd
                }),
            )
            .with_body(result.lines().map(|l| l.to_string()).collect());

            let _ = tx.send(TaskUpdate::Complete {
                id,
                feed_item: card,
            });
        });

        Some(id)
    }

    /// Check for completed tasks and return feed items.
    pub fn tick(&mut self) -> Vec<FeedItem> {
        let mut items = Vec::new();
        while let Ok(update) = self.task_rx.try_recv() {
            match update {
                TaskUpdate::Complete { id, feed_item } => {
                    if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                        task.status = TaskStatus::Completed("done".to_string());
                    }
                    items.push(feed_item);
                }
                TaskUpdate::Failed { id, error } => {
                    if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                        task.status = TaskStatus::Failed(error.clone());
                    }
                    let card = FeedItem::new(
                        FeedSource::Task,
                        Priority::Normal,
                        "Task Failed".to_string(),
                    )
                    .with_body(vec![error]);
                    items.push(card);
                }
            }
        }
        items
    }

    /// Count of currently running tasks.
    pub fn active_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Running))
            .count()
    }

    /// Count of completed tasks.
    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Completed(_)))
            .count()
    }

    /// Get (active, completed) counts.
    pub fn counts(&self) -> (usize, usize) {
        (self.active_count(), self.completed_count())
    }

    /// Get active tasks for sidebar display.
    pub fn active_tasks(&self) -> Vec<(&str, u64)> {
        self.tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::Running))
            .map(|t| (t.name.as_str(), t.started.elapsed().as_secs()))
            .collect()
    }

    /// Summary string for sidebar.
    pub fn summary(&self) -> String {
        let active = self.active_count();
        let done = self.completed_count();
        if active == 0 && done == 0 {
            "(none)".to_string()
        } else {
            format!("{} active, {} done", active, done)
        }
    }
}
