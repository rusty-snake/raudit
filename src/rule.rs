/*
 * Copyright © 2021 rusty-snake
 *
 * This file is part of raudit
 *
 * raudit is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * raudit is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use anyhow::anyhow;
use log::{error, trace, warn};
use nix::errno::Errno;
use nix::libc;
use nix::unistd::{access, AccessFlags, Uid};
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::say;
use crate::utils::expand_tilde;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Rule {
    Caps,
    NewPrivs,
    Print(String),
    Read(PathBuf),
    Write(PathBuf),
}
impl Rule {
    pub fn check(&self) {
        match self {
            Self::Caps => ensure_nocaps(),
            Self::NewPrivs => ensure_nonewprivs(),
            Self::Print(msg) => println!("{}", msg),
            Self::Read(path) => ensure_noread(path),
            Self::Write(path) => ensure_nowrite(path),
        }
    }
}
impl FromStr for Rule {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "caps" {
            Ok(Self::Caps)
        } else if s == "newprivs" {
            Ok(Self::NewPrivs)
        } else if let Some(msg) = s.strip_prefix("print ") {
            Ok(Self::Print(msg.to_string()))
        } else if let Some(path) = s.strip_prefix("read ") {
            Ok(Self::Read(expand_tilde(path).into_owned()))
        } else if let Some(path) = s.strip_prefix("write ") {
            Ok(Self::Write(expand_tilde(path).into_owned()))
        } else {
            Err(anyhow!("Invalid rule: {}", s))
        }
    }
}

fn ensure_nocaps() {
    crate::SCORE_MAX.with(|score_max| score_max.set(score_max.get() + 2));

    if fs::read_to_string("/proc/self/status")
        .expect("Failed to read /proc/self/status")
        .lines()
        .find(|line| line.starts_with("CapBnd:"))
        .expect(
            "/proc/self/status does not contain 'CapBnd', are you running a kernel below 2.6.26?",
        )
        .eq("CapBnd:\t0000000000000000")
    {
        say!(GREAT: "The capability bounding set is empty.");
    } else {
        say!(BAD: "The capability bounding set is NOT empty.");
    }
}

fn ensure_nonewprivs() {
    crate::SCORE_MAX.with(|score_max| score_max.set(score_max.get() + 2));

    const NULL: libc::c_ulong = 0;

    match unsafe { libc::prctl(libc::PR_GET_NO_NEW_PRIVS, NULL, NULL, NULL, NULL) } {
        0 => {
            say!(BAD:
                "no_new_privs is NOT set, the sandbox can acquire new privileges using execve."
            )
        }
        1 => {
            say!(GREAT:
                "no_new_privs is set, the sandbox can not acquire new privileges using execve."
            )
        }
        _ => error!("Failed to get nnp state: {}", io::Error::last_os_error()),
    }
}

fn ensure_noread(path: &Path) {
    crate::SCORE_MAX.with(|score_max| score_max.set(score_max.get() + 1));

    if path.is_relative() {
        warn!(
            "Relative paths aren't expected to work: read {}",
            path.display()
        );
    }

    if let Err(err) = access(path, AccessFlags::R_OK) {
        match err.as_errno() {
            Some(Errno::EACCES) => {
                say!(GOOD: "The sandbox cannot read {}.", path.display());
            }
            Some(Errno::ENOENT) => {
                say!(GOOD:
                    "The sandbox cannot read {} because it does not exist.",
                    path.display(),
                );
            }
            _ => {
                error!("Failed to check read access to {}: {}", path.display(), err);
            }
        }
    } else {
        say!(UGLY: "The sandbox can read {}.", path.display());
    }
}

fn ensure_nowrite(path: &Path) {
    crate::SCORE_MAX.with(|score_max| score_max.set(score_max.get() + 1));

    if path.is_relative() {
        warn!(
            "Relative paths aren't expected to work: write {}",
            path.display()
        );
    }

    for parent in path.ancestors() {
        trace!("ensure_nowrite: parent = {}", parent.display());
        if let Err(err) = access(parent, AccessFlags::W_OK) {
            match err.as_errno() {
                Some(Errno::EACCES) => {
                    if fs::metadata(path).unwrap().uid() == Uid::current().as_raw() {
                        say!(UGLY: "The sandbox can write to {} after a chmod.", path.display());
                    } else {
                        say!(GOOD: "The sandbox cannot write to {}.", path.display());
                    }
                }
                Some(Errno::EROFS) => {
                    say!(GOOD: "The sandbox cannot write to {}.", path.display());
                }
                Some(Errno::ENOENT) => {
                    // This is the only place where we want to continue this loop.
                    // Let's use `continue' here and add a catch-all `break' at the end.
                    continue;
                }
                _ => {
                    error!(
                        "Failed to check write access to {} at {}: {}",
                        path.display(),
                        parent.display(),
                        err,
                    );
                }
            }
        } else {
            if path == parent {
                say!(UGLY: "The sandbox can write to {}.", path.display());
            } else {
                if *crate::REAL_HOME {
                    say!(UGLY: "The sandbox can create {}.", path.display());
                } else {
                    say!(GOOD: "The sandbox cannot write to {}.", path.display());
                }
            }
        }
        // Catch-all `break'.
        // ENOENT is the only case where we want to continue, see above.
        break;
    }
}
