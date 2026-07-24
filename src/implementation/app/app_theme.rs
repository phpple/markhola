#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AppTheme {
    Default,
    Github,
    Dark,
    Light,
}

impl AppTheme {
    pub(crate) const ALL: [AppTheme; 4] = [
        AppTheme::Default,
        AppTheme::Github,
        AppTheme::Dark,
        AppTheme::Light,
    ];

    pub(crate) const fn key(self) -> &'static str {
        match self {
            AppTheme::Default => "default",
            AppTheme::Github => "github",
            AppTheme::Dark => "dark",
            AppTheme::Light => "light",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            AppTheme::Default => "Default",
            AppTheme::Github => "GitHub",
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
        }
    }

    pub(crate) fn from_key(value: &str) -> Option<Self> {
        match value {
            "default" => Some(AppTheme::Default),
            "github" => Some(AppTheme::Github),
            "dark" => Some(AppTheme::Dark),
            "light" => Some(AppTheme::Light),
            _ => None,
        }
    }
}
