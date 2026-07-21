use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};

pub(super) type AssetAccessRegistry = Arc<RwLock<HashMap<u64, PathBuf>>>;

pub(super) fn new_registry() -> AssetAccessRegistry {
    Arc::new(RwLock::new(HashMap::new()))
}

pub(super) fn register_document(registry: &AssetAccessRegistry, document_id: u64, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Document path does not have a parent directory.".to_string())?;
    let root = std::fs::canonicalize(parent)
        .map_err(|error| format!("Failed to resolve document directory: {error}"))?;
    registry
        .write()
        .map_err(|_| "Local asset registry is unavailable.".to_string())?
        .insert(document_id, root);
    Ok(())
}

pub(super) fn unregister_document(registry: &AssetAccessRegistry, document_id: u64) {
    if let Ok(mut entries) = registry.write() {
        entries.remove(&document_id);
    }
}

pub(super) fn resolve_asset(
    registry: &AssetAccessRegistry,
    document_id: u64,
    relative_path: &str,
) -> Result<PathBuf, AssetAccessError> {
    let root = registry
        .read()
        .map_err(|_| AssetAccessError::Unavailable)?
        .get(&document_id)
        .cloned()
        .ok_or(AssetAccessError::UnknownDocument)?;
    let path = Path::new(relative_path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
        })
    {
        return Err(AssetAccessError::Forbidden);
    }
    let resolved = std::fs::canonicalize(root.join(path)).map_err(AssetAccessError::Io)?;
    if !resolved.starts_with(&root) {
        return Err(AssetAccessError::Forbidden);
    }
    Ok(resolved)
}

#[derive(Debug)]
pub(super) enum AssetAccessError {
    Forbidden,
    UnknownDocument,
    Unavailable,
    Io(std::io::Error),
}

impl AssetAccessError {
    pub(super) fn status_code(&self) -> u16 {
        match self {
            Self::Forbidden | Self::UnknownDocument => 403,
            Self::Unavailable => 503,
            Self::Io(error) if error.kind() == std::io::ErrorKind::NotFound => 404,
            Self::Io(_) => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("markhola-asset-access-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn resolves_assets_only_within_the_registered_document_directory() {
        let root = temp_dir("within-root");
        let document = root.join("note.md");
        let image = root.join("image.png");
        std::fs::write(&document, "# Note").unwrap();
        std::fs::write(&image, "image").unwrap();
        let registry = new_registry();
        register_document(&registry, 7, &document).unwrap();

        assert_eq!(resolve_asset(&registry, 7, "image.png").unwrap(), std::fs::canonicalize(&image).unwrap());
        assert!(matches!(resolve_asset(&registry, 7, "../outside.png"), Err(AssetAccessError::Forbidden)));
        assert!(matches!(resolve_asset(&registry, 7, "/etc/passwd"), Err(AssetAccessError::Forbidden)));

        unregister_document(&registry, 7);
        assert!(matches!(resolve_asset(&registry, 7, "image.png"), Err(AssetAccessError::UnknownDocument)));
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_that_escape_the_registered_directory() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("symlink-root");
        let outside = temp_dir("symlink-outside");
        let document = root.join("note.md");
        let secret = outside.join("secret.png");
        std::fs::write(&document, "# Note").unwrap();
        std::fs::write(&secret, "secret").unwrap();
        symlink(&secret, root.join("escape.png")).unwrap();
        let registry = new_registry();
        register_document(&registry, 8, &document).unwrap();

        assert!(matches!(resolve_asset(&registry, 8, "escape.png"), Err(AssetAccessError::Forbidden)));

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(outside);
    }
}
