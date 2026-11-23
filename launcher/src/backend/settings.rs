//! The launcher's settings (`settings.json` in the launcher's data folder): what the Settings
//! page changes. Nothing here decides what the club decides in `club.toml` (`config.rs`): the
//! build, the servers, the game's first options.
use super::*;

/// The launcher's appearance, as older launchers saved it: a theme is one look now, and a saved
/// `light` opens on the Taiga theme (`Settings::parse`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    /// Follow the system's light or dark.
    System,
}

/// What the launcher's window does while the game runs, as the player picks it in Settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhilePlaying {
    /// The window stays where it is.
    #[default]
    Stay,
    /// The window goes to the Dock or the taskbar, and comes back when the game closes.
    Minimize,
    /// The launcher hides (macOS only: elsewhere it minimises), and comes back when the game
    /// closes.
    Hide,
}

impl WhilePlaying {
    pub const ALL: [WhilePlaying; 3] = [Self::Stay, Self::Minimize, Self::Hide];

    /// Its name in `settings.json` and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Stay => "stay",
            Self::Minimize => "minimize",
            Self::Hide => "hide",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        let name = match name.as_str() {
            "minimise" => "minimize",
            "keep" | "open" | "off" => "stay",
            n => n,
        };
        Self::ALL.into_iter().find(|w| w.name() == name)
    }

    /// What this computer does for it: only macOS can hide a whole app.
    pub fn here(self) -> Self {
        match self {
            Self::Hide if !cfg!(target_os = "macos") => Self::Minimize,
            w => w,
        }
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    /// Only read to move an old light appearance to the Taiga theme (`parse`): always dark now,
    /// a theme being one look.
    #[serde(default)]
    pub appearance: Appearance,
    /// The look's flourishes, each on or off (`effects`): glow, gradients, the hover shine.
    #[serde(default)]
    pub effects: crate::effects::Effects,
    /// What the window does while the game runs. Stays by default.
    #[serde(default)]
    pub while_playing: WhilePlaying,
    /// The accent colour on top of the appearance (`themes::Accent`). Horizon's violet by default.
    #[serde(default)]
    pub accent: crate::themes::Accent,
    /// How much white the glass look's plates take, in percent (the mockup's Glass, 0 to 16).
    /// Unset is the mockup's 6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glass: Option<f32>,
    /// How much the glass look's plates blur the scene behind them, in px (0 to 48). Unset is
    /// the mockup's 24.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blur: Option<f32>,
    /// The launcher theme (`themes::theme`): its sky behind the light appearance. Unset is
    /// Horizon's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// The cheaper scene for older computers (a lower-resolution sky, a box blur, the scene
    /// following the mouse less often and resizes baked in the background). Unset is off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance: Option<bool>,
    /// Home's background: a picture the player picked, kept in the data folder under this name
    /// (`themes::keep_background`). Unset shows Horizon's landscape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// The window's language. Unset is the club's (`club.toml`), which the supervisor set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<crate::i18n::Language>,
    /// The accent picked under each launcher theme, by its id (`themes::HORIZON` for Horizon's
    /// own), so going back to a theme brings back its colour. A launcher older than this map
    /// drops it when it saves: the themes then only start from their own accents again.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub theme_accents: std::collections::BTreeMap<String, crate::themes::Accent>,
    /// Reduced motion: unset follows the system's, true keeps the launcher still, false lets it
    /// move even when the system asks for less.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce_motion: Option<bool>,
}

impl Settings {
    /// The saved settings.
    pub fn load() -> Self {
        Self::load_from(&data_dir().join("settings.json"))
    }

    pub(super) fn load_from(path: &Path) -> Self {
        let text = fs::read_to_string(path).unwrap_or_default();
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Self {
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else {
            return Self::default();
        };
        if let Some(fields) = value.as_object_mut() {
            // An appearance from a newer launcher is Dark, not a reason to lose every setting.
            if fields
                .get("appearance")
                .is_some_and(|a| serde_json::from_value::<Appearance>(a.clone()).is_err())
            {
                fields.remove("appearance");
            }
            if fields
                .get("while_playing")
                .is_some_and(|w| serde_json::from_value::<WhilePlaying>(w.clone()).is_err())
            {
                fields.remove("while_playing");
            }
            if fields.get("accent").is_some_and(|a| {
                serde_json::from_value::<crate::themes::Accent>(a.clone()).is_err()
            }) {
                fields.remove("accent");
            }
            // A glass or blur that isn't a number is the default, not a reason to lose the rest.
            for key in ["glass", "blur"] {
                if fields.get(key).is_some_and(|v| !v.is_number()) {
                    fields.remove(key);
                }
            }
            look_settings::tidy(fields);
            // A language this launcher doesn't have is the club's.
            if fields.get("language").is_some_and(|l| {
                serde_json::from_value::<Option<crate::i18n::Language>>(l.clone()).is_err()
            }) {
                fields.remove("language");
            }
        }
        let mut settings: Self = serde_json::from_value(value).unwrap_or_default();
        look_settings::settle(&mut settings);
        settings
    }
    /// The window's language: the one picked in Settings, else the club's (`club.toml`).
    pub fn language(&self, club: crate::config::Language) -> crate::i18n::Language {
        self.language.unwrap_or_else(|| club.of())
    }
    /// The plates' white as a fraction (0 to 0.16): [`glass`](Self::glass) in percent.
    pub fn glass_tint(&self) -> f32 {
        self.glass
            .filter(|v| v.is_finite())
            .map_or(horizon_theme::glass::TINT, |v| v.clamp(0., 16.) / 100.)
    }
    /// The plates' blur in px (0 to 48).
    pub fn glass_blur(&self) -> f32 {
        self.blur
            .filter(|v| v.is_finite())
            .map_or(horizon_theme::glass::BLUR, |v| v.clamp(0., 48.))
    }
    /// Whether the cheaper scene is on ([`performance`](Self::performance)).
    pub fn performance(&self) -> bool {
        self.performance.unwrap_or(false)
    }
    pub fn save(&self) -> Result<(), String> {
        // The window and background jobs share the process's temporary file.
        static SAVE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = SAVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // A write that stops halfway leaves settings.json unreadable, and `load` answers
        // defaults for it: the file is replaced only once it is whole.
        let dir = data_dir();
        let path = dir.join("settings.json");
        let tmp = dir.join(format!("settings.{}.tmp", std::process::id()));
        let json =
            serde_json::to_vec_pretty(self).map_err(|e| format!("{}: {e}", path.display()))?;
        let written = fs::create_dir_all(&dir)
            .and_then(|()| fs::write(&tmp, json))
            .and_then(|()| fs::rename(&tmp, &path));
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(format!("{}: {e}", path.display()));
        }
        Ok(())
    }
}
