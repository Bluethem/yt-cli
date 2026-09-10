use crate::{Result, YtcliError};
use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn which_bin(name: &str) -> Result<()> {
    let candidate = Path::new(name);
    let found = if candidate.components().count() > 1 {
        is_executable(candidate)
    } else {
        env::var_os("PATH")
            .map(|path| {
                env::split_paths(&path)
                    .map(|dir| dir.join(name))
                    .any(|candidate| is_executable(&candidate))
            })
            .unwrap_or(false)
    };

    if found {
        Ok(())
    } else {
        Err(YtcliError::MissingBinary(name.into()))
    }
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn explicit_path_must_be_an_executable_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tool");
        File::create(&path).unwrap();

        assert!(matches!(
            which_bin(path.to_str().unwrap()),
            Err(YtcliError::MissingBinary(_))
        ));

        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(which_bin(path.to_str().unwrap()).is_ok());
    }

    #[test]
    fn shell_syntax_is_not_executed() {
        let name = "definitely-missing; true";
        assert!(matches!(
            which_bin(name),
            Err(YtcliError::MissingBinary(missing)) if missing == name
        ));
    }
}
