use std::collections::{HashMap, VecDeque};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::aurora_client;
use crate::brain_client;
use crate::feed::{FeedItem, FeedSource, Priority, WidgetData};
use crate::telemetry::{AlertKind, SysTelemetry, TelemetryHistory};
use crate::ui::BlockColor;

/// The proactive engine generates feed items from background monitoring.
pub struct ProactiveEngine {
    pub telemetry_history: TelemetryHistory,
    feed_tx: mpsc::Sender<FeedItem>,
    /// Cooldowns: prevent the same alert kind from firing too frequently.
    cooldowns: HashMap<AlertKind, Instant>,
    cooldown_duration: Duration,
    /// World model polling state.
    world_model_interval: Duration,
    last_world_model_check: Instant,
    prediction_errors: VecDeque<f64>,
    last_world_model_card: Instant,
    world_model_cooldown: Duration,
    /// Track if cfcd is reachable.
    cfcd_available: Option<bool>,
    /// Brain proactive polling state.
    brain_proactive_interval: Duration,
    last_brain_proactive: Instant,
    /// Recent alert labels for brain context.
    recent_alert_labels: VecDeque<String>,
    /// Last user query for brain context.
    last_user_query: String,
    /// Session start time.
    session_start: Instant,
    /// Active/completed task counts for brain context.
    task_active: usize,
    task_completed: usize,
    /// User interest topics from session context.
    user_topics: Vec<String>,
}

impl ProactiveEngine {
    pub fn new(feed_tx: mpsc::Sender<FeedItem>) -> Self {
        Self {
            telemetry_history: TelemetryHistory::new(30),
            feed_tx,
            cooldowns: HashMap::new(),
            cooldown_duration: Duration::from_secs(60),
            world_model_interval: Duration::from_secs(15),
            last_world_model_check: Instant::now(),
            prediction_errors: VecDeque::new(),
            last_world_model_card: Instant::now(),
            world_model_cooldown: Duration::from_secs(60),
            cfcd_available: None,
            brain_proactive_interval: Duration::from_secs(120),
            last_brain_proactive: Instant::now(),
            recent_alert_labels: VecDeque::new(),
            last_user_query: String::new(),
            session_start: Instant::now(),
            task_active: 0,
            task_completed: 0,
            user_topics: Vec::new(),
        }
    }

    /// Called every telemetry refresh (2s). Updates history and checks for alerts.
    pub fn tick(&mut self, telemetry: &SysTelemetry) {
        self.telemetry_history.push(telemetry.clone());

        // Check telemetry thresholds
        let alerts = self.telemetry_history.check_thresholds();
        for alert in alerts {
            if let Some(last) = self.cooldowns.get(&alert.kind) {
                if last.elapsed() < self.cooldown_duration {
                    continue;
                }
            }
            self.cooldowns.insert(alert.kind.clone(), Instant::now());

            // Track alert label for brain context
            let label = alert.kind.label().to_string();
            self.recent_alert_labels.push_back(label.clone());
            if self.recent_alert_labels.len() > 10 {
                self.recent_alert_labels.pop_front();
            }

            let card = FeedItem::new(
                FeedSource::System,
                alert.priority,
                label,
            )
            .with_body(vec![alert.message]);

            let _ = self.feed_tx.send(card);
        }

        // World model check (every 15s, non-blocking via thread)
        if self.last_world_model_check.elapsed() >= self.world_model_interval {
            self.last_world_model_check = Instant::now();
            self.check_world_model();
        }

        // Brain proactive check (every 120s, non-blocking via thread)
        if self.last_brain_proactive.elapsed() >= self.brain_proactive_interval {
            self.last_brain_proactive = Instant::now();
            self.check_brain_proactive(telemetry);
        }
    }

    /// Query the world model in a background thread.
    fn check_world_model(&mut self) {
        let tx = self.feed_tx.clone();
        let can_send_card = self.last_world_model_card.elapsed() >= self.world_model_cooldown;
        let prev_errors: Vec<f64> = self.prediction_errors.iter().copied().collect();
        let was_available = self.cfcd_available;

        // Clone what we need for the thread
        let feed_tx = tx;

        std::thread::spawn(move || {
            match aurora_client::query_prediction() {
                Ok(insight) => {
                    // Determine if this is interesting enough to show
                    let error = insight.prediction_error;

                    // Check if cfcd just became available
                    if was_available == Some(false) || was_available.is_none() {
                        let card = FeedItem::new(
                            FeedSource::WorldModel,
                            crate::feed::Priority::Normal,
                            "World Model Online".to_string(),
                        )
                        .with_body(vec![
                            format!(
                                "CFC-JEPA model active (weights: {})",
                                insight.weight_version
                            ),
                            format!(
                                "Predictions: {} | Learning: {}",
                                insight.total_predictions,
                                if insight.learning_enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                }
                            ),
                        ]);
                        let _ = feed_tx.send(card);
                        return;
                    }

                    if !can_send_card {
                        return;
                    }

                    // Trend analysis: is error rising?
                    if prev_errors.len() >= 5 {
                        let recent_avg: f64 =
                            prev_errors.iter().rev().take(3).sum::<f64>() / 3.0;
                        let older_avg: f64 = prev_errors.iter().take(3).sum::<f64>() / 3.0;

                        if error > 0.6 && recent_avg > older_avg * 1.3 {
                            let card = FeedItem::new(
                                FeedSource::WorldModel,
                                crate::feed::Priority::Normal,
                                "System Becoming Unpredictable".to_string(),
                            )
                            .with_body(vec![
                                format!(
                                    "Prediction error: {:.2} (rising from {:.2})",
                                    error, older_avg
                                ),
                                "The world model is detecting unusual system behavior.".to_string(),
                            ]);
                            let _ = feed_tx.send(card);
                        } else if error < 0.2 && recent_avg < 0.25 && older_avg > 0.4 {
                            let card = FeedItem::new(
                                FeedSource::WorldModel,
                                crate::feed::Priority::Low,
                                "System Stable".to_string(),
                            )
                            .with_body(vec![
                                format!("Prediction error: {:.2} (decreasing)", error),
                                "The world model has learned your usage patterns.".to_string(),
                            ]);
                            let _ = feed_tx.send(card);
                        }
                    }
                }
                Err(_) => {
                    // cfcd not available — only report once
                    if was_available == Some(true) {
                        let card = FeedItem::new(
                            FeedSource::WorldModel,
                            crate::feed::Priority::Normal,
                            "World Model Offline".to_string(),
                        )
                        .with_body(vec![
                            "CFC-JEPA model is not responding.".to_string(),
                        ]);
                        let _ = feed_tx.send(card);
                    }
                }
            }
        });

        // Try to get a synchronous quick check for tracking
        match aurora_client::query_prediction() {
            Ok(insight) => {
                self.prediction_errors.push_back(insight.prediction_error);
                if self.prediction_errors.len() > 20 {
                    self.prediction_errors.pop_front();
                }
                self.cfcd_available = Some(true);
            }
            Err(_) => {
                if self.cfcd_available == Some(true) {
                    self.cfcd_available = Some(false);
                } else if self.cfcd_available.is_none() {
                    self.cfcd_available = Some(false);
                }
            }
        }
    }

    /// Record a user query for brain proactive context.
    pub fn set_last_query(&mut self, query: &str) {
        self.last_user_query = query.to_string();
    }

    /// Update task counts for brain context.
    pub fn set_task_counts(&mut self, active: usize, completed: usize) {
        self.task_active = active;
        self.task_completed = completed;
    }

    /// Update user interest topics from session context.
    pub fn set_user_topics(&mut self, topics: Vec<String>) {
        self.user_topics = topics;
    }

    /// Query the brain proactive endpoint in a background thread.
    fn check_brain_proactive(&self, telemetry: &SysTelemetry) {
        let feed_tx = self.feed_tx.clone();

        // Build context
        let mem_pct = if telemetry.mem_total_mb > 0 {
            let used = telemetry.mem_total_mb.saturating_sub(telemetry.mem_avail_mb);
            (used as f64 / telemetry.mem_total_mb as f64) * 100.0
        } else {
            0.0
        };

        let uptime_secs = telemetry.uptime_secs;
        let uptime_str = if uptime_secs >= 3600 {
            format!("{}h{}m", uptime_secs / 3600, (uptime_secs % 3600) / 60)
        } else {
            format!("{}m{}s", uptime_secs / 60, uptime_secs % 60)
        };

        let telem_ctx = brain_client::TelemetryContext {
            cpu: telemetry.cpu_percent,
            mem_pct,
            uptime: uptime_str,
            procs: telemetry.num_procs,
            network: telemetry.ip_addr.clone(),
        };

        // World model context from prediction errors
        let wm_ctx = if !self.prediction_errors.is_empty() {
            let last_error = *self.prediction_errors.back().unwrap_or(&0.0);
            let trend = if self.prediction_errors.len() >= 3 {
                let recent: f64 = self.prediction_errors.iter().rev().take(3).sum::<f64>() / 3.0;
                let older: f64 = self.prediction_errors.iter().take(3).sum::<f64>() / 3.0;
                if recent > older * 1.2 { "rising".to_string() }
                else if recent < older * 0.8 { "falling".to_string() }
                else { "stable".to_string() }
            } else {
                "unknown".to_string()
            };
            Some(brain_client::WorldModelContext {
                prediction_error: last_error,
                trend,
                learning_enabled: true,
            })
        } else {
            None
        };

        let recent_alerts: Vec<String> = self.recent_alert_labels.iter().cloned().collect();

        let session_secs = self.session_start.elapsed().as_secs();
        let session_str = if session_secs >= 3600 {
            format!("{}h{}m", session_secs / 3600, (session_secs % 3600) / 60)
        } else {
            format!("{}m", session_secs / 60)
        };

        let user_ctx = Some(brain_client::UserActivityContext {
            last_query: self.last_user_query.clone(),
            session_duration: session_str,
        });

        let task_ctx = if self.task_active > 0 || self.task_completed > 0 {
            Some(brain_client::TaskContext {
                active: self.task_active,
                completed: self.task_completed,
            })
        } else {
            None
        };

        let context = brain_client::ProactiveContext {
            telemetry: Some(telem_ctx),
            world_model: wm_ctx,
            recent_alerts,
            user_activity: user_ctx,
            tasks: task_ctx,
        };

        std::thread::spawn(move || {
            match brain_client::query_brain_proactive(&context) {
                Ok(resp) if resp.has_insight && !resp.text.is_empty() => {
                    let priority = match resp.priority.as_str() {
                        "urgent" => Priority::Urgent,
                        "low" => Priority::Low,
                        _ => Priority::Normal,
                    };

                    let title = match resp.category.as_str() {
                        "suggestion" => "Suggestion",
                        "warning" => "Warning",
                        _ => "Insight",
                    };

                    let mut card = FeedItem::new(
                        FeedSource::Brain,
                        priority,
                        title.to_string(),
                    )
                    .with_body(resp.text.lines().map(|l| l.to_string()).collect());

                    // Add widgets if present
                    if let Some(first_widget) = resp.widgets.first() {
                        let color = match first_widget.widget_type.as_str() {
                            "weather" => BlockColor::Yellow,
                            "system" => BlockColor::Green,
                            _ => BlockColor::Cyan,
                        };
                        card = card.with_widget(WidgetData {
                            widget_type: first_widget.widget_type.clone(),
                            title: first_widget.title.clone(),
                            lines: first_widget.lines.clone(),
                            color,
                        });
                    }

                    let _ = feed_tx.send(card);
                }
                _ => {} // No insight or error — silently skip
            }
        });
    }

    /// Get CPU history for sparkline rendering.
    pub fn cpu_history(&self) -> Vec<f64> {
        self.telemetry_history.cpu_history()
    }

    /// Get memory percent history for sparkline rendering.
    pub fn mem_pct_history(&self) -> Vec<f64> {
        self.telemetry_history.mem_pct_history()
    }
}
