use zed_extension_api::{self as zed, Command, Extension, LanguageServerId, Result, Worktree};

struct TuneExtension;

impl Extension for TuneExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let command = worktree.which("dyno").ok_or_else(|| {
            "dyno was not found on PATH; install Dyno or add it to PATH".to_owned()
        })?;
        Ok(Command {
            command,
            args: vec!["lsp".to_owned()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(TuneExtension);
