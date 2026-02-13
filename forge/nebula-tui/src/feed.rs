use std::time::Instant;

use crate::ui::BlockColor;

/// Source of a feed item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedSource {
    System,
    Brain,
    WorldModel,
    User,
    Task,
}

impl FeedSource {
    /// Single-character icon for the source.
    pub fn icon(&self) -> &'static str {
        match self {
            FeedSource::System => "S",
            FeedSource::Brain => "B",
            FeedSource::WorldModel => "W",
            FeedSource::User => "U",
            FeedSource::Task => "T",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            FeedSource::System => "System",
            FeedSource::Brain => "Brain",
            FeedSource::WorldModel => "World Model",
            FeedSource::User => "User",
            FeedSource::Task => "Task",
        }
    }

    pub fn color(&self) -> BlockColor {
        match self {
            FeedSource::System => BlockColor::Green,
            FeedSource::Brain => BlockColor::Yellow,
            FeedSource::WorldModel => BlockColor::Cyan,
            FeedSource::User => BlockColor::Blue,
            FeedSource::Task => BlockColor::White,
        }
    }
}

/// Priority level for feed items.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Urgent = 0,
    Normal = 1,
    Low = 2,
}

/// Widget data embedded in a feed item.
#[derive(Clone, Debug)]
pub struct WidgetData {
    pub widget_type: String,
    pub title: String,
    pub lines: Vec<String>,
    pub color: BlockColor,
}

/// A single item in the feed.
#[derive(Clone, Debug)]
pub struct FeedItem {
    pub id: u64,
    pub source: FeedSource,
    pub priority: Priority,
    pub title: String,
    pub body: Vec<String>,
    pub widget: Option<WidgetData>,
    pub timestamp: Instant,
    pub seen: bool,
    pub stale_after_secs: Option<u64>,
    pub collapsed: bool,
    pub dismissed: bool,
    /// If set, a new item from this source auto-replaces the previous one.
    pub replaces_source: Option<FeedSource>,
}

impl FeedItem {
    pub fn new(source: FeedSource, priority: Priority, title: String) -> Self {
        Self {
            id: 0, // assigned by FeedStore
            source,
            priority,
            title,
            body: Vec::new(),
            widget: None,
            timestamp: Instant::now(),
            seen: false,
            stale_after_secs: None,
            collapsed: false,
            dismissed: false,
            replaces_source: None,
        }
    }

    pub fn with_body(mut self, lines: Vec<String>) -> Self {
        self.body = lines;
        self
    }

    pub fn with_widget(mut self, widget: WidgetData) -> Self {
        self.widget = Some(widget);
        self
    }

    pub fn with_stale(mut self, secs: u64) -> Self {
        self.stale_after_secs = Some(secs);
        self
    }

    pub fn with_replaces(mut self, source: FeedSource) -> Self {
        self.replaces_source = Some(source);
        self
    }

    /// Whether this item has expired.
    pub fn is_stale(&self) -> bool {
        if let Some(secs) = self.stale_after_secs {
            self.timestamp.elapsed().as_secs() >= secs
        } else {
            false
        }
    }

    /// Human-readable relative timestamp.
    pub fn age_str(&self) -> String {
        let secs = self.timestamp.elapsed().as_secs();
        if secs < 5 {
            "just now".to_string()
        } else if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else {
            format!("{}h ago", secs / 3600)
        }
    }
}

/// The feed store holds all feed items with capping and pruning.
pub struct FeedStore {
    items: Vec<FeedItem>,
    next_id: u64,
    max_items: usize,
}

impl FeedStore {
    pub fn new(max_items: usize) -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
            max_items,
        }
    }

    /// Push a new item, assigning it an ID. Handles auto-replacement.
    pub fn push(&mut self, mut item: FeedItem) {
        // Handle replacement: dismiss the most recent item from the same source
        if let Some(ref replace_source) = item.replaces_source {
            for existing in self.items.iter_mut().rev() {
                if &existing.source == replace_source && !existing.dismissed {
                    existing.dismissed = true;
                    break;
                }
            }
        }

        item.id = self.next_id;
        self.next_id += 1;
        self.items.push(item);

        // Cap total items
        if self.items.len() > self.max_items {
            // Remove oldest dismissed items first, then oldest items
            if let Some(pos) = self.items.iter().position(|i| i.dismissed) {
                self.items.remove(pos);
            } else {
                self.items.remove(0);
            }
        }
    }

    /// Get visible (non-dismissed, non-stale) items in chronological order.
    pub fn visible_items(&self) -> Vec<&FeedItem> {
        self.items
            .iter()
            .filter(|i| !i.dismissed && !i.is_stale())
            .collect()
    }

    /// Mark an item as seen.
    pub fn mark_seen(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.seen = true;
        }
    }

    /// Toggle collapsed state for an item.
    pub fn toggle_collapse(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.collapsed = !item.collapsed;
        }
    }

    /// Dismiss an item (hide from view).
    pub fn dismiss(&mut self, id: u64) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.dismissed = true;
        }
    }

    /// Count of unseen, non-dismissed items.
    pub fn unseen_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| !i.seen && !i.dismissed && !i.is_stale())
            .count()
    }

    /// Count of unseen urgent items.
    pub fn unseen_urgent_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| !i.seen && !i.dismissed && !i.is_stale() && i.priority == Priority::Urgent)
            .count()
    }

    /// Prune stale items by marking them dismissed.
    pub fn prune_stale(&mut self) {
        for item in &mut self.items {
            if item.is_stale() && !item.dismissed {
                item.dismissed = true;
            }
        }
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Total visible count.
    pub fn visible_count(&self) -> usize {
        self.visible_items().len()
    }
}
