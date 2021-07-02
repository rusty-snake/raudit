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

//! Module for various programming helpers not directly related to raudit.

use anyhow::anyhow;
use nix::libc;
use nix::unistd::isatty;
use std::borrow::Cow;
use std::path::Path;
use termcolor::ColorChoice;

/// Performs a tilde expansion.
///
/// If `path` starts with `~/`, the expansion is performed and [`Cow::Owned`](std::borrow::Cow::Owned)
/// with a path starting with [`struct@crate::HOME`] is returned. If it
/// does not start with `~/`, [`Cow::Borrowed`](std::borrow::Cow::Borrowed)
/// is returned with an unchanged path.
///
/// # Examples
///
/// ```ignore
/// # use std::path::{Path, Pathbuf};
/// assert_eq!(expand_tilde("~/.ssh"), Cow::Owned(PathBuf::from("/home/user/.ssh")));
/// assert_eq!(expand_tilde("/root/~/.ssh"), Cow::Borrowed(Path::new("/root/~/.ssh")));
/// assert_eq!(expand_tilde(Path::new("~/.ssh")), Cow::Owned(PathBuf::from("/home/user/.ssh")));
/// assert_eq!(expand_tilde(&PathBuf::from("/etc/passwd")), Cow::Borrowed(Path::new("/etc/passwd")));
/// ```
pub fn expand_tilde<P: AsRef<Path> + ?Sized>(path: &P) -> Cow<'_, Path> {
    expand_tilde_impl(path.as_ref())
}
#[doc(hidden)]
#[inline(always)]
fn expand_tilde_impl(path: &Path) -> Cow<'_, Path> {
    if let Ok(stripped_path) = path.strip_prefix("~/") {
        Cow::Owned(crate::HOME.join(stripped_path))
    } else {
        Cow::Borrowed(path)
    }
}

/// Helper function used by structopt to parse `--color=<color-choice>`
pub fn parse_color_choice(color_choice: &str) -> anyhow::Result<ColorChoice> {
    match color_choice {
        "always" => Ok(ColorChoice::Always),
        "ansi" => Ok(ColorChoice::AlwaysAnsi),
        "auto" => {
            if isatty(libc::STDOUT_FILENO)? {
                Ok(ColorChoice::Auto)
            } else {
                Ok(ColorChoice::Never)
            }
        }
        "never" => Ok(ColorChoice::Never),
        invalid_choice => Err(anyhow!(
            "Invalid color choice '{}'. Valid choices are 'always', 'ansi', 'auto' and 'never'.",
            invalid_choice
        )),
    }
}
