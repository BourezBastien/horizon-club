//! A student's first Play (docs/school-edition/launcher-target.md, section 8). When the game home
//! holds no `options.txt` yet, the launcher writes it, then the game's two other settings files:
//! the club's values and the game's language, the two switches that skip vanilla's first-start
//! screens, and Horizon's "Performance" preset, so weak computers start quick. The game reads
//! such files, keeps their values and completes them itself. From then on they are the student's,
//! who may change anything in the game's own screens, and the launcher never writes them again.

use crate::config::{Game, Language};
use std::path::Path;

/// The game's options, in its home.
pub const OPTIONS: &str = "options.txt";
/// Horizon's graphics settings (the game's `graphics.rs`), beside the options.
pub const GRAPHICS: &str = "horizon-graphics.txt";
/// Horizon's own switches (the game's `extras.rs`), beside the options.
pub const EXTRAS: &str = "horizon-extras.txt";

/// The game's `options.txt` as a student starts with it: `[game]`'s values, the game's language
/// (`lang`), vanilla's two first-start screens skipped (`onboardAccessibility`,
/// `skipMultiplayerWarning`), and what the "Performance" preset sets among vanilla's options
/// (`graphics_preset.rs`), the render and simulation distances aside, which are the club's.
pub fn options(game: &Game, language: Language) -> String {
    let lang = match language {
        Language::French => "fr_fr",
        Language::English => "en_us",
    };
    format!(
        "renderDistance:{}\n\
         simulationDistance:{}\n\
         maxFps:{}\n\
         lang:{lang}\n\
         onboardAccessibility:false\n\
         skipMultiplayerWarning:true\n\
         renderClouds:\"false\"\n\
         particles:2\n\
         entityShadows:false\n\
         biomeBlendRadius:0\n\
         mipmapLevels:0\n\
         entityDistanceScaling:0.5\n\
         guiScale:0\n\
         fullscreen:false\n",
        game.render_distance, game.simulation_distance, game.max_fps
    )
}

/// The game's `horizon-graphics.txt`: what `apply_preset(Preset::Performance)` sets among that
/// file's keys, in the order the game writes them (`Graphics::serialize`). A key the preset
/// doesn't set keeps the game's default, and the game completes the file itself.
pub const PERFORMANCE: &str = "preset:performance\n\
     lodDistance:0\n\
     caveCulling:true\n\
     entityCulling:async\n\
     particleCulling:async\n\
     opaqueIce:false\n\
     blockEntityCacheShulkerBoxes:true\n\
     blockEntityCacheSigns:true\n\
     blockEntityCacheDecoratedPots:true\n\
     blockEntityCacheBells:true\n\
     blockEntityCacheChests:true\n\
     blockEntityCacheBanners:true\n\
     blockEntityCacheCopperGolemStatues:true\n\
     chunkUpdates:0\n\
     uploadBudgetKiB:0\n\
     sky:true\n\
     fog:true\n\
     weather:true\n\
     animatedTextures:true\n\
     ditherCutoutMips:false\n\
     resolutionScale:100\n\
     dynamicResolution:false\n\
     fxaa:false\n\
     taa:false\n\
     upscaling:native\n\
     gpuCulling:auto\n\
     msaa4x:false\n";

/// The game's `horizon-extras.txt`: every AI feature hidden and off (`horizon_core::no_ai`).
pub const NO_AI: &str = "noAi:true\n";

/// Writes the three files into `home` when it holds no `options.txt` yet: false when there was
/// nothing to write. `options.txt` goes last, as its being there is what says the student has
/// played before. Err: why a file could not be written.
pub fn write_missing(home: &Path, game: &Game, language: Language) -> Result<bool, String> {
    let path = |file: &str| home.join(file);
    match std::fs::metadata(path(OPTIONS)) {
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(why(&path(OPTIONS), &e)),
    }
    // The game home may not exist yet at a student's very first Play.
    std::fs::create_dir_all(home).map_err(|e| why(home, &e))?;
    let write =
        |file: &str, text: &str| std::fs::write(path(file), text).map_err(|e| why(&path(file), &e));
    write(GRAPHICS, PERFORMANCE)?;
    write(EXTRAS, NO_AI)?;
    write(OPTIONS, &options(game, language))?;
    Ok(true)
}

/// What the window says of a file of the game's it could not write.
fn why(path: &Path, error: &std::io::Error) -> String {
    let error = format!("{}: {error}", path.display());
    crate::i18n::trf(
        "The game's options could not be written: {error}",
        &[("error", &error)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lines of a `key:value` file.
    fn lines(text: &str) -> Vec<(&str, &str)> {
        text.lines()
            .map(|line| line.split_once(':').expect("key:value"))
            .collect()
    }

    #[test]
    fn the_options_hold_the_clubs_values_and_skip_the_first_start_screens() {
        // The club's defaults, in French.
        let defaults = options(&Game::default(), Language::French);
        assert_eq!(
            defaults,
            "renderDistance:8\n\
             simulationDistance:5\n\
             maxFps:60\n\
             lang:fr_fr\n\
             onboardAccessibility:false\n\
             skipMultiplayerWarning:true\n\
             renderClouds:\"false\"\n\
             particles:2\n\
             entityShadows:false\n\
             biomeBlendRadius:0\n\
             mipmapLevels:0\n\
             entityDistanceScaling:0.5\n\
             guiScale:0\n\
             fullscreen:false\n"
        );
        // The supervisor's values, and English for a club that isn't French: only those four
        // lines differ.
        let club = Game {
            render_distance: 6,
            simulation_distance: 7,
            max_fps: 30,
        };
        let theirs = options(&club, Language::English);
        assert_eq!(
            lines(&theirs)[..4],
            [
                ("renderDistance", "6"),
                ("simulationDistance", "7"),
                ("maxFps", "30"),
                ("lang", "en_us")
            ]
        );
        assert_eq!(lines(&theirs)[4..], lines(&defaults)[4..]);
    }

    /// The graphics file is the Performance preset: every line is `key:value`, each key once,
    /// and the values are those `apply_preset(Preset::Performance)` gives the game's `Graphics`.
    #[test]
    fn the_graphics_file_is_the_performance_preset() {
        assert_eq!(
            lines(PERFORMANCE),
            [
                ("preset", "performance"),
                ("lodDistance", "0"),
                ("caveCulling", "true"),
                ("entityCulling", "async"),
                ("particleCulling", "async"),
                ("opaqueIce", "false"),
                ("blockEntityCacheShulkerBoxes", "true"),
                ("blockEntityCacheSigns", "true"),
                ("blockEntityCacheDecoratedPots", "true"),
                ("blockEntityCacheBells", "true"),
                ("blockEntityCacheChests", "true"),
                ("blockEntityCacheBanners", "true"),
                ("blockEntityCacheCopperGolemStatues", "true"),
                ("chunkUpdates", "0"),
                ("uploadBudgetKiB", "0"),
                ("sky", "true"),
                ("fog", "true"),
                ("weather", "true"),
                ("animatedTextures", "true"),
                ("ditherCutoutMips", "false"),
                ("resolutionScale", "100"),
                ("dynamicResolution", "false"),
                ("fxaa", "false"),
                ("taa", "false"),
                ("upscaling", "native"),
                ("gpuCulling", "auto"),
                ("msaa4x", "false"),
            ]
        );
        assert!(PERFORMANCE.ends_with('\n'));
        // No AI, on.
        assert_eq!(NO_AI, "noAi:true\n");
        assert_eq!(lines(NO_AI), [("noAi", "true")]);
    }

    #[test]
    fn the_files_are_written_once_and_only_while_the_options_are_missing() {
        let home = std::env::temp_dir().join(format!("horizon-first-play-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        // The game home may not exist yet: it is made.
        assert!(write_missing(&home, &Game::default(), Language::French).unwrap());
        let read = |file: &str| std::fs::read_to_string(home.join(file)).unwrap();
        assert_eq!(read(OPTIONS), options(&Game::default(), Language::French));
        assert_eq!(read(GRAPHICS), PERFORMANCE);
        assert_eq!(read(EXTRAS), NO_AI);
        // A student changed their settings in the game: the launcher leaves every file alone,
        // a deleted one too.
        std::fs::write(home.join(OPTIONS), "renderDistance:12\nmaxFps:120\n").unwrap();
        std::fs::write(home.join(GRAPHICS), "preset:ultra\n").unwrap();
        std::fs::remove_file(home.join(EXTRAS)).unwrap();
        assert!(!write_missing(&home, &Game::default(), Language::French).unwrap());
        assert_eq!(read(OPTIONS), "renderDistance:12\nmaxFps:120\n");
        assert_eq!(read(GRAPHICS), "preset:ultra\n");
        assert!(!home.join(EXTRAS).exists());
        let _ = std::fs::remove_dir_all(&home);
    }
}
