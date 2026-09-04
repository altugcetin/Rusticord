use std::ffi::OsString;
use std::path::PathBuf;

pub const APPLICATION_IDENTIFIER: &str = "ist.alchm.rusticord";

pub fn data_directory() -> Option<PathBuf> {
    data_directory_from_env(
        std::env::var_os("APPDATA"),
        std::env::var_os("HOME"),
        std::env::var_os("XDG_DATA_HOME"),
    )
}

#[cfg(windows)]
pub fn data_directory_from_env(
    appdata: Option<OsString>,
    _home: Option<OsString>,
    _xdg_data_home: Option<OsString>,
) -> Option<PathBuf> {
    appdata.map(|value| PathBuf::from(value).join("Rusticord"))
}

#[cfg(target_os = "macos")]
pub fn data_directory_from_env(
    _appdata: Option<OsString>,
    home: Option<OsString>,
    _xdg_data_home: Option<OsString>,
) -> Option<PathBuf> {
    home.map(|value| {
        PathBuf::from(value)
            .join("Library")
            .join("Application Support")
            .join("Rusticord")
    })
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn data_directory_from_env(
    _appdata: Option<OsString>,
    home: Option<OsString>,
    xdg_data_home: Option<OsString>,
) -> Option<PathBuf> {
    if let Some(xdg) = xdg_data_home {
        return Some(PathBuf::from(xdg).join("rusticord"));
    }
    home.map(|value| {
        PathBuf::from(value)
            .join(".local")
            .join("share")
            .join("rusticord")
    })
}

#[cfg(test)]
mod tests {
    use super::{APPLICATION_IDENTIFIER, data_directory_from_env};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn identifier_has_three_dns_labels() {
        let mut labels = APPLICATION_IDENTIFIER.split('.');
        assert_eq!(labels.next(), Some("ist"));
        assert_eq!(labels.next(), Some("alchm"));
        assert_eq!(labels.next(), Some("rusticord"));
        assert_eq!(labels.next(), None);
    }

    #[test]
    fn windows_data_directory_uses_appdata() {
        if !cfg!(windows) {
            return;
        }
        let path = data_directory_from_env(
            Some(OsString::from("C:\\Users\\me\\AppData\\Roaming")),
            None,
            None,
        );
        assert_eq!(
            path,
            Some(PathBuf::from("C:\\Users\\me\\AppData\\Roaming\\Rusticord"))
        );
    }

    #[test]
    fn unix_data_directory_prefers_xdg() {
        if cfg!(windows) || cfg!(target_os = "macos") {
            return;
        }
        let path = data_directory_from_env(
            None,
            Some(OsString::from("/home/me")),
            Some(OsString::from("/tmp/data")),
        );
        assert_eq!(path, Some(PathBuf::from("/tmp/data/rusticord")));
    }
}
