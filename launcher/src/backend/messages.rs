use super::*;

/// What the threads that start and run the game tell the window (`Launcher::drain`).
pub enum Msg {
    /// A line of the game's output.
    Line(String),
    /// The game's process ended: how it ended badly.
    Exit(Result<(), String>),
    /// The step a start of the game is on (`play_step::say`), already in the launcher's language.
    PlayStep(String),
    /// A start of the game is prepared, off the window's thread: the game's command, ready to
    /// start ([`Staged`]), or why not.
    GamePrepared(Box<Result<Staged, String>>),
}
