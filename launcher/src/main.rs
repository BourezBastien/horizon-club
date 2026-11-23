//! Horizon Club launcher: lists the club's game builds and starts the game, or a server a
//! student hosts (docs/school-edition/launcher-target.md). What the club decides is in its
//! `club.toml` (`horizon_launcher::config`); the launcher downloads and prepares nothing.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use horizon_launcher_scenery::{art, glass, rain, scene, sky};
mod blocks_chrome;
mod blocks_play;
mod blocks_settings;
mod blocks_versions;
mod launcher_messages;
mod launcher_play;
mod launcher_render;
mod launcher_setup;
mod widgets;
pub(crate) use widgets::*;
mod blocks;
mod console_page;
mod cull;
mod frame;
mod glass_home;
mod glass_home_lists;
mod glass_home_tabs;
mod glass_palette;
mod glass_settings;
mod glass_toast;
mod glass_ui;
mod glass_versions;
mod glide;
mod home;
mod home_tabs;
mod horizon_mark;
mod language_page;
mod line_edit;
mod motion;
mod multiplayer_page;
mod nav_strip;
mod perf;
mod picture_cache;
mod plate;
mod play_step;
mod scrollbar;
mod servers_page;
mod shell;
mod specimen;
mod theme;
mod themes_page;
mod titlebar;
mod update_page;
mod while_playing;
mod window_root;
mod worlds_page;

use gpui::{
    actions, div, ease_out_quint, prelude::*, px, relative, size, Animation, AnimationExt,
    AnyElement, App, Bounds, ClickEvent, Context, Div, ElementId, FocusHandle, FontWeight, Hsla,
    KeyBinding, KeyDownEvent, Menu, MenuItem, MouseDownEvent, PathPromptOptions, Pixels, Point,
    SharedString, Stateful, Subscription, Window, WindowBounds, WindowOptions,
};
use horizon_launcher::backend::{self, Msg, Running, Settings};
use horizon_launcher::i18n::{self, tr, trf};
use scrollbar::Scrollbar;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use theme::Theme;

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Play,
    Versions,
    Servers,
    Settings,
}

impl Page {
    /// `HORIZON_LAUNCHER_PAGE=servers` (or `versions`, `settings`) opens on that page, for
    /// screenshots; Home otherwise (`worlds`...: Home on that tab, `HomeTab::from_env`).
    fn from_env() -> Page {
        let page = match std::env::var("HORIZON_LAUNCHER_PAGE").as_deref() {
            Ok("servers") => Page::Servers,
            Ok("versions") => Page::Versions,
            Ok("settings") => Page::Settings,
            _ => Page::Play,
        };
        page
    }

    /// Every page, with its name and icon: the top bar shows all but Settings, which has its
    /// own button.
    const ALL: [(Page, &'static str, &'static str); 4] = [
        (Page::Play, "Home", "home"),
        (Page::Servers, "Servers", "server"),
        (Page::Versions, "Versions", "archive"),
        (Page::Settings, "Settings", "sliders"),
    ];

    /// The pages the top bar shows: all but Settings. `hosting` is the club's `[hosting]
    /// enabled`: the Servers page exists only when a student may host.
    fn in_top_bar(hosting: bool) -> impl Iterator<Item = (Page, &'static str, &'static str)> {
        Self::ALL.into_iter().filter(move |(page, ..)| {
            !matches!(page, Page::Settings) && (hosting || *page != Page::Servers)
        })
    }
}

/// The last game the launcher started ended badly: what it left behind.
#[derive(Clone)]
struct Crash {
    /// The crash report the run wrote, if it wrote one.
    report: Option<std::path::PathBuf>,
    /// One line on what went wrong: the report's error, else how the process ended.
    summary: String,
    copied: bool,
}

/// Why the game can't start now: the sentence Home shows, and the rail's few words for it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Refusal {
    said: String,
    rail: String,
}

/// While Home says the game can't start, the club's file and the club's builds are read again
/// this often: what the supervisor fixed shows without restarting the launcher.
const RECHECK: Duration = Duration::from_secs(5);

/// The opening: the wordmark holds at least `OPENING_HOLD` (until the release
/// list is in, at most `OPENING_MAX`), then the launcher appears over `OPENING_REVEAL`.
const OPENING_HOLD: Duration = horizon_theme::OPENING_HOLD;
const OPENING_REVEAL: Duration = horizon_theme::OPENING_REVEAL;

/// The phase of the opening at one moment: how far the reveal has gone, 0 to 1.
struct Opening {
    /// Set on the first frame, so a slow start does not eat into the hold.
    started: Option<Instant>,
    reveal: Option<Instant>,
    focus: FocusHandle,
}

fn ease_out_cubic(x: f32) -> f32 {
    horizon_theme::ease_out_cubic(x)
}

const LOG_LINES: usize = 2000;

struct Launcher {
    theme: Theme,
    page: Page,
    /// The page under the glass Settings dialog: the last one shown before it opened.
    behind: Page,
    /// How many times the page changed, for the page transition.
    switches: usize,
    content_scroll: Scrollbar,
    /// The top bar's page tabs, including their horizontal overflow controls.
    nav_strip: nav_strip::NavStrip,
    /// The glass Settings page's own scroll: its sections beside their list.
    settings_pane: gpui::ScrollHandle,
    /// The glass Settings page's search, its focus and whether it has it.
    settings_query: line_edit::LineEdit,
    settings_query_focus: FocusHandle,
    settings_query_focused: bool,
    /// The section the glass Settings list marked when last drawn (`glass_settings`).
    settings_marked: usize,
    /// The rows of the page's long list (Versions) last drawn.
    long_list: cull::Cull,
    opening: Option<Opening>,
    /// The builds of the club's `core\` as last listed, newest first: at the window's start, and
    /// whenever the club's file is read again (`reload_club`).
    installed: Vec<String>,
    /// Settings' any-colour swatch shows the hue strip (the glass look).
    accent_strip: bool,
    /// Settings' Glass and Blur sliders (the glass look).
    look_sliders: themes_page::LookSliders,
    /// Where the accent picker's hue strip is on screen (`themes_page.rs`), to map a click to a
    /// hue.
    hue_area: std::rc::Rc<std::cell::Cell<Option<Bounds<gpui::Pixels>>>>,
    /// A drag on that strip is in flight, so a release anywhere writes the accent down.
    hue_dragging: bool,
    /// Settings > Language's dropdown (`language_page.rs`).
    language_menu: bool,
    /// The glass sidebar's ⋮ menu (`frame.rs`).
    row_menu: frame::RowMenu,
    /// The rail's hover: whether the pointer is on it, and since when (it eases open and shut).
    rail: frame::Rail,
    /// The glass look's colours for the player's accent (`horizon_theme::glass`).
    look: horizon_theme::glass::Glass,
    /// Where the mouse went down to close a menu by pressing outside it, so that the
    /// same press, ending on the menu's own button, doesn't open it again.
    menu_closed_at: Option<Point<Pixels>>,
    /// Work the look out again on the next frame (a launcher theme put on or taken off).
    restyle: bool,
    /// Why the last picture for Home's background was refused.
    background_error: Option<String>,
    settings: Settings,
    /// The last failed settings write, kept across page changes until a save succeeds.
    settings_save_error: Option<String>,
    /// The club's configuration (`club.toml`), as it was last read well: its name, its servers,
    /// the build it plays. The default one until then, and while the file can't be read.
    config: horizon_launcher::config::Config,
    /// Why `club.toml` could not be read, when it can't: Home says so, and Play does nothing.
    config_error: Option<String>,
    /// Why the game can't start now, when it can't. None when Play would start the game.
    refusal: Option<Refusal>,
    /// When the club's file and the club's builds were last read again (`reload_club`).
    checked: Option<Instant>,
    /// Lines the game printed since the launcher started: the Console tab's. It only grows, so
    /// it moves when a line arrives even while it is full and its length stays.
    console_log: VecDeque<String>,
    log_revision: u64,
    client: Option<Running>,
    client_error: Option<String>,
    /// When the running client started, to tell its crash report from older ones.
    client_started: Option<std::time::SystemTime>,
    crash: Option<Crash>,
    /// The Servers page (`servers_page.rs`).
    servers: servers_page::Servers,
    /// The window's own focus, so Tab reaches the controls when nothing else has the keyboard.
    /// The palette (⌘K), while open (`glass_palette`).
    palette: Option<glass_palette::Palette>,
    /// The toast last drawn (`glass_toast`), by its number.
    toast_seen: Option<u64>,
    root_focus: FocusHandle,
    /// Who plays, named by the Windows session (`identity`): the login folded to a Minecraft
    /// name, or none when that login can't make one, or belongs to a session several students
    /// share (Home says why, and Play does nothing).
    player: Option<String>,
    /// The game is starting: the build it plays, from Play until its process runs or the start
    /// failed.
    starting_client: Option<String>,
    /// The window's width at the last frame: Home's head lays out by it, as the mockup's `xl`.
    window_width: f32,
    /// The window's longer side in points, and the screen's pixels per point: the size pictures
    /// are made at (`picture_cache`).
    window_long_side: f32,
    scale_factor: f32,
    /// Where the next start goes at once: `--world <folder>` or `--server <address>`.
    quick_play: Option<(&'static str, String)>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    _appearance: Subscription,
    /// Redraws when the window gains or loses focus: the sky moves only in front.
    _activation: Subscription,
    /// The session's measurements (`perf.rs`), and when the steps they time began.
    perf: perf::Perf,
    /// The tab open under Home's hero (`home_tabs.rs`).
    home_tab: home_tabs::HomeTab,
    /// Home's Play or folder menu, while open (glass look).
    home_menu: glass_home::HomeMenu,
    /// Whether the pointer is over Home's folder button, which then shows its ⋯.
    folder_hovered: bool,
    /// The list row under the pointer in Home's glass tabs (a world, a server): it shows its
    /// actions then.
    list_hover: Option<SharedString>,
    /// A Home list row's ⋯ menu, while open, and whether its Delete… was clicked once.
    list_menu: Option<(SharedString, bool)>,
    /// The size of Home's tab strip, measured as it is laid out: the glass panel's plate cuts
    /// its corner around it.
    home_strip: std::rc::Rc<std::cell::Cell<Option<gpui::Size<Pixels>>>>,
    /// When the running game was first seen running, for Home's "Playing for…".
    client_since: Option<Instant>,
    play_pressed: Option<Instant>,
    /// The step the start in progress said last (`play_step`).
    play_step: play_step::PlayStep,
    /// A benchmark run (`HORIZON_LAUNCHER_BENCH`) started this window to measure it.
    bench: Option<perf::Bench>,
    /// Horizon's own mark as images, by size in device pixels (`horizon_mark.rs`).
    marks: std::collections::HashMap<u32, Option<std::sync::Arc<gpui::RenderImage>>>,
    /// Home's Worlds tab (`worlds_page.rs`).
    worlds: worlds_page::Worlds,
    multiplayer: multiplayer_page::Multiplayer,
    /// Pictures from disk made at their drawn size (`picture_cache`).
    pictures: picture_cache::Pictures,
    console: console_page::Console,
    /// What the window did when the game started (`while_playing.rs`).
    aside: while_playing::Aside,
    /// The player's servers, for the sidebar until the Servers page has read them.
    side_servers: Vec<horizon_launcher::servers::Entry>,
    /// The launcher's update (`update_page.rs`): asked of a session that may change the share.
    update: update_page::Update,
    /// The pixel art made for this window (`art.rs`), and pictures downloaded by URL.
    sky: gpui::Entity<sky::SkyView>,
    /// The pixel sunset of the new design (dark appearance): its own view, so a frame of it
    /// alone redraws nothing else.
    scene: gpui::Entity<scene::Scene>,
    /// Hypr Neon's rain over its black night (`rain.rs`).
    rain: gpui::Entity<rain::Rain>,
    /// What is behind the launcher, drawn as its sibling (`window_root.rs`).
    backdrop: gpui::Entity<window_root::Backdrop>,
    /// What the views behind the launcher were last handed.
    pushed: Option<launcher_render::Pushed>,
    /// The window's scale, as of its last frame, for art drawn outside `render`'s reach.
    scale: f32,
    /// The window's title as it was last set: the club's name (`club.toml`).
    title: Option<String>,
}

actions!(horizon_launcher, [Quit, CloseWindow]);

/// What closing the last window does: stop the servers this launcher started, exit. A game that
/// is running keeps running. Cmd+Q goes through here too.
fn shut_down(cx: &mut App) {
    servers_page::stop_on_quit();
    cx.quit()
}

/// The macOS shortcuts: Cmd+Q quits, Cmd+W closes the window.
#[cfg(target_os = "macos")]
fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
    ]
}

#[cfg(not(target_os = "macos"))]
fn key_bindings() -> Vec<KeyBinding> {
    Vec::new()
}

/// The menu bar on macOS, where "Quit" belongs to the app menu.
fn menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "Horizon Launcher".into(),
            items: vec![MenuItem::action(tr("Quit Horizon Launcher"), Quit)],
            disabled: false,
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Close Window", CloseWindow)],
            disabled: false,
        },
    ]
}

mod startup;

fn main() {
    startup::run();
}

/// Whether the click `event` began with the mouse-down that closed a menu at `closed_at`.
/// A keyboard click never did.
fn closed_by_press(closed_at: Option<Point<Pixels>>, event: &ClickEvent) -> bool {
    matches!(event, ClickEvent::Mouse(click) if closed_at == Some(click.down.position))
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_menu_button_stays_closed_after_the_press_that_closed_its_menu() {
        use super::closed_by_press;
        use gpui::{point, px, ClickEvent, KeyboardClickEvent, MouseClickEvent, MouseDownEvent};
        let click = |x: f32| {
            ClickEvent::Mouse(MouseClickEvent {
                down: MouseDownEvent {
                    position: point(px(x), px(20.)),
                    ..MouseDownEvent::default()
                },
                ..MouseClickEvent::default()
            })
        };
        let closed_at = Some(point(px(10.), px(20.)));
        // The press on the button that closed the open menu: it stays closed.
        assert!(closed_by_press(closed_at, &click(10.)));
        // A later press, or one with no menu closed, toggles as before.
        assert!(!closed_by_press(closed_at, &click(11.)));
        assert!(!closed_by_press(None, &click(10.)));
        let keyboard = ClickEvent::Keyboard(KeyboardClickEvent::default());
        assert!(!closed_by_press(closed_at, &keyboard));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn cmd_q_quits_and_cmd_w_closes_the_window() {
        use super::{key_bindings, CloseWindow, Quit};
        use gpui::{Keymap, Keystroke};
        let keymap = Keymap::new(key_bindings());
        let key = |s: &'static str| [Keystroke::parse(s).unwrap()];
        let (hit, _) = keymap.bindings_for_input(&key("cmd-q"), &[]);
        assert!(hit.iter().any(|b| b.action().partial_eq(&Quit)));
        let (hit, _) = keymap.bindings_for_input(&key("cmd-w"), &[]);
        assert!(hit.iter().any(|b| b.action().partial_eq(&CloseWindow)));
    }

    #[test]
    fn the_app_menu_has_quit() {
        let menus = super::menus();
        assert_eq!(menus[0].name, "Horizon Launcher");
        assert!(matches!(
            &menus[0].items[0],
            gpui::MenuItem::Action { name, .. } if name == "Quit Horizon Launcher"
        ));
    }
}
