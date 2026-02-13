use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ui::ActivePanel;

/// Actions that the app can perform in response to input.
pub enum AppAction {
    // Global
    Quit,
    SwitchPanel,
    ReturnToInput,

    // Input panel
    TypeChar(char),
    Backspace,
    Delete,
    Submit,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    HistoryUp,
    HistoryDown,

    // Feed panel
    FeedSelectPrev,
    FeedSelectNext,
    FeedToggleCollapse,
    FeedDismiss,
    FeedPageUp,
    FeedPageDown,

    // Global scrolling (works from input panel too)
    PageUp,
    PageDown,

    // Global shortcuts
    TriggerSysinfo,
    TriggerWorldModel,

    // No-op
    Noop,
}

/// Route a key event to an action based on the active panel.
pub fn route(key: KeyEvent, panel: &ActivePanel, thinking: bool) -> AppAction {
    // Global: Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return AppAction::Quit;
    }

    // Global shortcuts
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') => return AppAction::TriggerSysinfo,
            KeyCode::Char('w') => return AppAction::TriggerWorldModel,
            _ => {}
        }
    }

    match panel {
        ActivePanel::Input => route_input(key, thinking),
        ActivePanel::Feed => route_feed(key),
        ActivePanel::Sidebar => route_sidebar(key),
    }
}

fn route_input(key: KeyEvent, thinking: bool) -> AppAction {
    match key.code {
        KeyCode::Tab => AppAction::SwitchPanel,
        KeyCode::Enter if !thinking => AppAction::Submit,
        KeyCode::Backspace if !thinking => AppAction::Backspace,
        KeyCode::Delete if !thinking => AppAction::Delete,
        KeyCode::Left => AppAction::CursorLeft,
        KeyCode::Right => AppAction::CursorRight,
        KeyCode::Home => AppAction::CursorHome,
        KeyCode::End => AppAction::CursorEnd,
        KeyCode::Up => AppAction::HistoryUp,
        KeyCode::Down => AppAction::HistoryDown,
        KeyCode::PageUp => AppAction::PageUp,
        KeyCode::PageDown => AppAction::PageDown,
        KeyCode::Char(c) if !thinking => AppAction::TypeChar(c),
        _ => AppAction::Noop,
    }
}

fn route_feed(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Tab => AppAction::SwitchPanel,
        KeyCode::Esc => AppAction::ReturnToInput,
        KeyCode::Up | KeyCode::Char('k') => AppAction::FeedSelectPrev,
        KeyCode::Down | KeyCode::Char('j') => AppAction::FeedSelectNext,
        KeyCode::Enter => AppAction::FeedToggleCollapse,
        KeyCode::Char('d') => AppAction::FeedDismiss,
        KeyCode::PageUp => AppAction::FeedPageUp,
        KeyCode::PageDown => AppAction::FeedPageDown,
        _ => AppAction::Noop,
    }
}

fn route_sidebar(key: KeyEvent) -> AppAction {
    match key.code {
        KeyCode::Tab => AppAction::SwitchPanel,
        KeyCode::Esc => AppAction::ReturnToInput,
        _ => AppAction::Noop,
    }
}
