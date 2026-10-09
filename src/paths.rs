use std::{io, path::PathBuf};

pub(crate) fn data_dir() -> io::Result<PathBuf> {
    let variable = |key| {
        std::env::var_os(key)
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::other(format!("{key} is not available")))
    };
    if cfg!(target_os = "windows") {
        Ok(variable("APPDATA")?.join("Yapshire"))
    } else if cfg!(target_os = "macos") {
        Ok(variable("HOME")?.join("Library/Application Support/Yapshire"))
    } else if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
        Ok(PathBuf::from(base).join("yapshire"))
    } else {
        Ok(variable("HOME")?.join(".local/share/yapshire"))
    }
}

pub(crate) fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let temp = path.with_extension(format!(
        "{}-{}.tmp",
        std::process::id(),
        rand::random::<u64>()
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
