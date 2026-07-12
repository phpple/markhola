use std::path::PathBuf;

pub(super) fn documentation_markdown_path() -> Option<PathBuf> {
    let candidates = [
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join("assets").join("help").join("Documentation.md")),
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .map(|dir| dir.join("../Resources/help/Documentation.md"))
        }),
        std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .and_then(|dir| dir.parent())
                .map(|contents| contents.join("Resources/help/Documentation.md"))
        }),
    ];

    candidates.into_iter().flatten().find(|path| path.exists())
}
