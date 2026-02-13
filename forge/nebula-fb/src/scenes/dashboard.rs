/// Dashboard — generative card layout from brain server.
/// Status bar (top), greeting + cards (middle), omnibar (bottom).

use crate::brain_client;
use crate::input::InputEvent;
use crate::layout;
use crate::renderer::Renderer;
use crate::scene::{Scene, Transition};
use crate::telemetry;
use crate::text::TextRenderer;
use crate::theme;
use crate::widgets::card::{self, CardData};
use crate::widgets::status_bar;
use crate::widgets::text_input::{self, TextInputState};

const SETUP_FILE: &str = "/tmp/aether_setup.json";
const TELEMETRY_INTERVAL_SECS: f32 = 5.0;
const DASHBOARD_REFRESH_SECS: f32 = 120.0;

pub struct Dashboard {
    screen_width: u32,
    screen_height: u32,
    greeting: String,
    subtitle: String,
    cards: Vec<CardData>,
    selected_card: usize,
    omnibar: TextInputState,
    telemetry: telemetry::TelemetryHistory,
    last_telemetry: f32,
    last_dashboard_refresh: f32,
    elapsed: f32,
    user_name: String,
    user_interests: Vec<String>,
    response_text: Option<String>,
    loading: bool,
}

impl Dashboard {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        // Load setup data
        let (name, interests) = load_setup();

        // Initial telemetry
        let mut history = telemetry::TelemetryHistory::new(60);
        let t = telemetry::read_telemetry();
        history.push(t.clone());

        // Determine greeting from time of day
        let hour = chrono::Local::now().hour();
        let tod = if hour < 12 {
            "morning"
        } else if hour < 17 {
            "afternoon"
        } else {
            "evening"
        };

        let greeting = format!("Good {}, {}.", tod, name);
        let subtitle = "Here's what I found for you today.".to_string();

        // Default cards (will be replaced by brain response)
        let cards = vec![
            CardData {
                card_type: "system".to_string(),
                title: "System Health".to_string(),
                body: None,
                metrics: Some(card::CardMetrics {
                    cpu: t.cpu_percent,
                    mem: t.mem_used_pct(),
                }),
                temp: None,
                desc: None,
                wind: None,
            },
            CardData {
                card_type: "text".to_string(),
                title: "Welcome".to_string(),
                body: Some("AetherOS is running. Ask me anything in the omnibar below.".to_string()),
                metrics: None,
                temp: None,
                desc: None,
                wind: None,
            },
        ];

        let mut dash = Self {
            screen_width,
            screen_height,
            greeting,
            subtitle,
            cards,
            selected_card: 0,
            omnibar: TextInputState::new("Ask me anything..."),
            telemetry: history,
            last_telemetry: 0.0,
            last_dashboard_refresh: -DASHBOARD_REFRESH_SECS, // trigger immediate refresh
            elapsed: 0.0,
            user_name: name,
            user_interests: interests,
            response_text: None,
            loading: false,
        };

        // Try to fetch initial dashboard from brain (non-blocking attempt)
        dash.try_refresh_dashboard();

        dash
    }

    fn try_refresh_dashboard(&mut self) {
        let t = self.telemetry.latest().cloned().unwrap_or_default();
        match brain_client::query_brain_dashboard(
            &self.user_name,
            &self.user_interests,
            t.cpu_percent,
            t.mem_used_pct(),
            &t.uptime_str(),
        ) {
            Ok(resp) => {
                if !resp.greeting.is_empty() {
                    self.greeting = resp.greeting;
                }
                if !resp.subtitle.is_empty() {
                    self.subtitle = resp.subtitle;
                }
                // Parse cards from JSON
                let mut new_cards = Vec::new();
                for card_val in &resp.cards {
                    if let Ok(cd) = serde_json::from_value::<CardData>(card_val.clone()) {
                        new_cards.push(cd);
                    }
                }
                if !new_cards.is_empty() {
                    self.cards = new_cards;
                }
            }
            Err(e) => {
                eprintln!("[dashboard] Brain dashboard error: {}", e);
            }
        }
        self.last_dashboard_refresh = self.elapsed;
    }

    fn submit_query(&mut self) {
        let query = self.omnibar.take_text();
        if query.is_empty() {
            return;
        }

        self.loading = true;
        match brain_client::query_brain(&query) {
            Ok(resp) => {
                self.response_text = Some(resp.text);
                self.loading = false;
            }
            Err(e) => {
                self.response_text = Some(format!("Error: {}", e));
                self.loading = false;
            }
        }
    }
}

use chrono::Timelike;

impl Scene for Dashboard {
    fn update(&mut self, dt: f32) -> Transition {
        self.elapsed += dt;

        // Refresh telemetry periodically
        if self.elapsed - self.last_telemetry >= TELEMETRY_INTERVAL_SECS {
            let t = telemetry::read_telemetry();
            self.telemetry.push(t.clone());
            self.last_telemetry = self.elapsed;

            // Update system card metrics
            for card in &mut self.cards {
                if card.card_type == "system" {
                    card.metrics = Some(card::CardMetrics {
                        cpu: t.cpu_percent,
                        mem: t.mem_used_pct(),
                    });
                }
            }
        }

        // Refresh dashboard from brain periodically
        if self.elapsed - self.last_dashboard_refresh >= DASHBOARD_REFRESH_SECS {
            self.try_refresh_dashboard();
        }

        Transition::None
    }

    fn draw(&self, renderer: &mut Renderer, text: &TextRenderer) {
        renderer.clear(theme::BG);

        let w = self.screen_width;
        let h = self.screen_height;

        // Status bar
        let t = self.telemetry.latest().cloned().unwrap_or_default();
        let time_str = chrono::Local::now().format("%I:%M %p").to_string();
        status_bar::draw_status_bar(
            renderer,
            text,
            &status_bar::StatusBarData {
                cpu_pct: t.cpu_percent,
                mem_pct: t.mem_used_pct(),
                net_status: t.ip_addr.clone(),
                time_str,
            },
            w,
        );

        // Greeting area
        let greeting_y = theme::STATUS_BAR_HEIGHT as f32 + 24.0;
        text.draw(
            renderer,
            &self.greeting,
            theme::CONTENT_MARGIN as f32,
            greeting_y,
            theme::FONT_SIZE_HEADING,
            theme::TEXT_PRIMARY,
        );
        text.draw(
            renderer,
            &self.subtitle,
            theme::CONTENT_MARGIN as f32,
            greeting_y + 32.0,
            theme::FONT_SIZE_BODY,
            theme::TEXT_SECONDARY,
        );

        // Card grid
        let card_top = theme::STATUS_BAR_HEIGHT + 90;
        let card_bottom = h - theme::OMNIBAR_HEIGHT - 20;

        // If we have a response, show it instead of cards
        if let Some(ref resp) = self.response_text {
            let resp_y = card_top as f32 + 16.0;
            let max_w = w as f32 - theme::CONTENT_MARGIN as f32 * 2.0;
            renderer.fill_rounded_rect(
                theme::CONTENT_MARGIN as f32,
                resp_y - 8.0,
                max_w,
                200.0,
                theme::CARD_RADIUS,
                theme::CARD,
            );
            renderer.stroke_rounded_rect(
                theme::CONTENT_MARGIN as f32,
                resp_y - 8.0,
                max_w,
                200.0,
                theme::CARD_RADIUS,
                theme::CARD_BORDER,
                1.0,
            );
            text.draw_wrapped(
                renderer,
                resp,
                theme::CONTENT_MARGIN as f32 + 16.0,
                resp_y + 8.0,
                max_w - 32.0,
                theme::FONT_SIZE_BODY,
                22.0,
                theme::TEXT_PRIMARY,
            );
        } else if self.loading {
            text.draw_centered(
                renderer,
                "Thinking...",
                0.0,
                (card_top + card_bottom) as f32 / 2.0,
                w as f32,
                theme::FONT_SIZE_BODY,
                theme::TEXT_MUTED,
            );
        } else {
            let slots = layout::card_grid(w, card_top, card_bottom, self.cards.len());
            for (i, (card_data, slot)) in self.cards.iter().zip(slots.iter()).enumerate() {
                card::draw_card(
                    renderer,
                    text,
                    card_data,
                    slot.x,
                    slot.y,
                    slot.w,
                    slot.h,
                    i == self.selected_card,
                );
            }
        }

        // Omnibar
        text_input::draw_omnibar(renderer, text, &self.omnibar, w, h);
    }

    fn handle_input(&mut self, event: InputEvent) -> Transition {
        match event {
            InputEvent::Char(ch) => {
                self.omnibar.insert_char(ch);
                self.response_text = None; // Clear response on new input
            }
            InputEvent::Backspace => {
                self.omnibar.backspace();
            }
            InputEvent::Enter => {
                self.submit_query();
            }
            InputEvent::Left => {
                self.omnibar.move_left();
            }
            InputEvent::Right => {
                self.omnibar.move_right();
            }
            InputEvent::Up => {
                if self.selected_card > 0 {
                    self.selected_card -= 1;
                }
            }
            InputEvent::Down => {
                if self.selected_card + 1 < self.cards.len() {
                    self.selected_card += 1;
                }
            }
            InputEvent::Tab => {
                self.response_text = None; // Clear response, show cards again
            }
            InputEvent::Escape => {
                self.response_text = None;
            }
            _ => {}
        }
        Transition::None
    }
}

fn load_setup() -> (String, Vec<String>) {
    match std::fs::read_to_string(SETUP_FILE) {
        Ok(data) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                let name = v
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("User")
                    .to_string();
                let interests: Vec<String> = v
                    .get("interests")
                    .and_then(|i| i.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                (name, interests)
            } else {
                ("User".to_string(), vec![])
            }
        }
        Err(_) => ("User".to_string(), vec![]),
    }
}
