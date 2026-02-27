use std::ffi::OsString;
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
    reject_symlink_path(path)?;
    fs::create_dir_all(path).map_err(|error| map_io_error(path, &error))?;
    set_private_directory_permissions(path)?;
    Ok(())
}

pub fn ledger_db_path(home: &Path) -> PathBuf {
    home.join("ledger.db")
}

pub fn open_connection(db_path: &Path) -> ClientResult<Connection> {
    reject_symlink_path(db_path)?;
    let connection =
        Connection::open(db_path).map_err(|error| map_sqlite_error(db_path, &error))?;
    connection
        .busy_timeout(Duration::from_millis(250))
        .map_err(|error| map_sqlite_error(db_path, &error))?;
    apply_writable_connection_pragmas(db_path, &connection)?;
    set_private_file_permissions(db_path)?;
    harden_sidecar_permissions(db_path)?;
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
fn set_private_directory_permissions(path: &Path) -> ClientResult<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| map_io_error(path, &error))?;
    assert_permissions(path, 0o700)
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> ClientResult<()> {
    Ok(())
}

fn apply_writable_connection_pragmas(db_path: &Path, connection: &Connection) -> ClientResult<()> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON;")
        .map_err(|error| map_sqlite_error(db_path, &error))
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> ClientResult<()> {
    use std::os::unix::fs::PermissionsExt;

    reject_symlink_path(path)?;
    let set_result = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    if let Err(error) = set_result {
        if error.kind() == std::io::ErrorKind::NotFound {
            return Ok(());
        }
        return Err(map_io_error(path, &error));
    }
    assert_permissions(path, 0o600)
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> ClientResult<()> {
    Ok(())
}

fn harden_sidecar_permissions(db_path: &Path) -> ClientResult<()> {
    let wal_path = db_path_with_suffix(db_path, "-wal");
    let shm_path = db_path_with_suffix(db_path, "-shm");
    let journal_path = db_path_with_suffix(db_path, "-journal");
    set_private_file_permissions(&wal_path)?;
    set_private_file_permissions(&shm_path)?;
    set_private_file_permissions(&journal_path)?;
    Ok(())
}

#[cfg(unix)]
fn assert_permissions(path: &Path, expected_mode: u32) -> ClientResult<()> {
    use std::io::{Error, ErrorKind};
    use std::os::unix::fs::PermissionsExt;

    let actual_mode = fs::metadata(path)
        .map_err(|error| map_io_error(path, &error))?
        .permissions()
        .mode()
        & 0o777;
    if actual_mode != expected_mode {
        let detail = format!(
            "Expected `{}` permissions {:o}, found {:o}.",
            path.display(),
            expected_mode,
            actual_mode
        );
        let error = Error::new(ErrorKind::PermissionDenied, detail);
        return Err(map_io_error(path, &error));
    }

    Ok(())
}

fn reject_symlink_path(path: &Path) -> ClientResult<()> {
    let parent = path.parent();
    for candidate in [Some(path), parent].into_iter().flatten() {
        let metadata = fs::symlink_metadata(candidate);
        if let Ok(meta) = metadata
            && meta.file_type().is_symlink()
        {
            return Err(ClientError::ledger_init_permission_denied(
                candidate,
                "Refusing to use a symlink path for ledger storage.",
            ));
        }
    }
    Ok(())
}

fn db_path_with_suffix(db_path: &Path, suffix: &str) -> PathBuf {
    let mut composed = OsString::from(db_path.as_os_str());
    composed.push(suffix);
    PathBuf::from(composed)
}
