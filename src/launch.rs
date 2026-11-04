use std::ffi::OsString;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitCode, Stdio};

use anyhow::{bail, Context, Result};

const RELEASE: &[u8; 8] = b"PRESIDE\n";

pub(crate) fn command(target: &Command) -> Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("__launch")
        .arg("--")
        .arg(target.get_program())
        .args(target.get_args());
    for (key, value) in target.get_envs() {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
    Ok(command)
}

pub(crate) fn release(child: &mut Child) -> Result<()> {
    child
        .stdin
        .take()
        .context("launch helper has no control pipe")?
        .write_all(RELEASE)
        .context("could not release launch helper")
}

pub(crate) fn exec(arguments: &[OsString]) -> Result<ExitCode> {
    let mut release = [0; RELEASE.len()];
    std::io::stdin()
        .read_exact(&mut release)
        .context("launch cancelled before runtime state was published")?;
    if &release != RELEASE {
        bail!("invalid launch authorization");
    }
    let (program, arguments) = arguments.split_first().context("missing launch command")?;
    let error = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .exec();
    Err(error).context("could not execute service")
}
