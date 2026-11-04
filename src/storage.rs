use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use directories::BaseDirs;
use fs2::FileExt;

pub struct MutationLock {
    _file: File,
}

impl MutationLock {
    pub fn acquire() -> Result<Self> {
        let home = BaseDirs::new().context("could not determine home directory")?;
        let directory = home.home_dir().join(".local/state/preside");
        fs::create_dir_all(&directory)?;
        let path = directory.join("mutation.lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&path)
            .with_context(|| format!("could not open operation lock {}", path.display()))?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { _file: file }),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        bail!("another Preside operation is still running; try again when it finishes");
                    }
                    thread::sleep(Duration::from_millis(25));
                }
                Err(error) => return Err(error).context("could not lock Preside operations"),
            }
        }
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let target = if path.is_symlink() {
        path.canonicalize()
            .context("could not resolve existing file symlink")?
    } else {
        path.to_path_buf()
    };
    let parent = target.parent().context("file has no parent directory")?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(&target)
        .map_err(|error| error.error)
        .with_context(|| format!("could not replace {}", target.display()))?;
    File::open(parent)?.sync_all()?;
    Ok(())
}
