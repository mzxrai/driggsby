use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, Error as SqliteError, OpenFlags, ffi::ErrorCode};

use crate::{ClientError, ClientResult};

pub fn resolve_ledger_home(home_override: Option<&Path>) -> ClientResult<PathBuf> {
    let candidate = match home_override {
        Some(path) => path.to_path_buf(),
        None => {
            if let Some(override_path) = std::env::var_os("DRIGGSBY_HOME") {
                PathBuf::from(override_path)
            } else if let Some(home_path) = home::home_dir() {
                home_path.join(".driggsby")
            } else {
                return Err(ClientError::ledger_init_failed(
                    Path::new("."),
                    "Could not resolve a home directory for ledger initialization.",
                ));
            }
        }
    };

    absolutize(&candidate)
}

pub fn ensure_ledger_directory(path: &Path) -> ClientResult<()> {
    fs::create_dir_all(path).map_err(|error| map_io_error(path, &error))?;
    set_private_permissions_best_effort(path);
    Ok(())
}

pub fn ledger_db_path(home: &Path) -> PathBuf {
    home.join("ledger.db")
}

pub fn open_connection(db_path: &Path) -> ClientResult<Connection> {
    let connection =
        Connection::open(db_path).map_err(|error| map_sqlite_error(db_path, &error))?;
    connection
        .busy_timeout(Duration::from_millis(250))
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    Ok(connection)
}

pub fn open_readonly_connection(db_path: &Path) -> ClientResult<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI;
    let connection = Connection::open_with_flags(db_path, flags)
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    connection
        .busy_timeout(Duration::from_millis(250))
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    Ok(connection)
}

pub fn map_io_error(path: &Path, error: &std::io::Error) -> ClientError {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        return ClientError::ledger_init_permission_denied(path, &error.to_string());
    }

    ClientError::ledger_init_failed(path, &error.to_string())
}

pub fn map_sqlite_error(path: &Path, error: &SqliteError) -> ClientError {
    let error_code = error.sqlite_error_code();

    if matches!(
        error_code,
        Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    ) {
        return ClientError::ledger_locked(path);
    }

    if matches!(error_code, Some(ErrorCode::NotADatabase)) {
        return ClientError::ledger_corrupt(path);
    }

    if matches!(
        error_code,
        Some(ErrorCode::CannotOpen | ErrorCode::ReadOnly)
    ) {
        return ClientError::ledger_init_permission_denied(path, &error.to_string());
    }

    ClientError::ledger_init_failed(path, &error.to_string())
}

fn absolutize(path: &Path) -> ClientResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .map_err(|error| ClientError::ledger_init_failed(path, &error.to_string()))
}

#[cfg(unix)]
fn set_private_permissions_best_effort(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn set_private_permissions_best_effort(_path: &Path) {}
