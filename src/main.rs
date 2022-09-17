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

#![allow(private_intra_doc_links)]
#![warn(rust_2018_idioms)]

use clap::Parser;
use lazy_static::lazy_static;
use log::{debug, error, trace, warn};
use nix::unistd::{access, AccessFlags};
use std::cell::Cell;
use std::env;
use std::io;
use std::io::BufRead;
use std::path::PathBuf;
use std::str::FromStr;
use stderrlog::StdErrLog;
use termcolor::ColorChoice;

mod rule;
mod utils;

use crate::rule::Rule;
use crate::utils::expand_tilde;

thread_local! {
    /// `thread_local!` The color choice for stderrlog and [`say`].
    ///
    /// Defaults to `never` and is overridden by `--color` from [`Opt`].
    static COLOR_CHOICE: Cell<ColorChoice> = Cell::new(ColorChoice::Never);

    /// `thread_local!` The score decrement counter.
    ///
    /// # How scoring works in raudit
    ///
    /// A rule can be of kind minor or major. A minor rule modifies the score by 1 and a major rule
    /// by 2. _Never_ mix minor and major for a rule, it will make the scoring inconsistent and
    /// broken.
    ///
    /// Every `crate::rule::ensure_*` must increase [`SCORE_MAX`] by 1 or 2 (depending on the rule
    /// kind) at first and later do a invocation of [`say`] with `UGLY:`/`GOOD:` if it is of kind
    /// minor or `BAD:`/`GREAT:` if it is of kind major. `say!(BAD: ...)`/`say!(UGLY: ...)` will
    /// then increase `SCORE_DEC`.
    ///
    /// TODO: `crate::rule::ensure_*` should only be called once for parameter-less rule / once per
    /// parameter.
    ///
    /// After all rules are handled, the actual score is computed by subtracting `SCORE_DEC` from
    /// `SCORE_MAX` (the maximal score that will happen if all check are GOOD/GREAT).
    static SCORE_DEC: Cell<u16> = Cell::new(0);
    /// `thread_local!` The score maximum counter.
    ///
    /// See [`SCORE_DEC`] for more information about how scoring works in raudit.
    static SCORE_MAX: Cell<u16> = Cell::new(0);
}

lazy_static! {
    /// `lazy_static!` The home directory of the current user.
    static ref HOME: PathBuf = {
        // allow deprecated here should be ok for now, the home crate
        // used by cargo/rustup and suggested by the rust documentation
        // (for a long time) does exactly the same for *nix systems.
        #[allow(deprecated)]
        env::home_dir()
            .expect("Failed to get user's home directory.")
    };

    /// `lazy_static!` Cached result of [`real_home`].
    static ref REAL_HOME: bool = real_home();
}

/// say a result to the user
///
/// # Invocations
///
/// A weighting followed by a [`format!`] like message. The last invocation
/// is considered an implementation detail. UGLY and BAD modify [`SCORE_DEC`]
/// in addition.
#[macro_export]
macro_rules! say {
    (GREAT: $($arg:tt)*) => {{
        say!(::termcolor::Color::Green, true, b"GREAT: ", $($arg)*);
    }};
    (GOOD: $($arg:tt)*) => {{
        say!(::termcolor::Color::Green, false, b"GOOD: ", $($arg)*);
    }};
    (MAYBE: $($arg:tt)*) => {{
        say!(::termcolor::Color::Yellow, false, b"MAYBE: ", $($arg)*);
    }};
    (UGLY: $($arg:tt)*) => {{
        crate::SCORE_DEC.with(|score_dec| score_dec.set(score_dec.get() + 1));
        say!(::termcolor::Color::Red, false, b"UGLY: ", $($arg)*);
    }};
    (BAD: $($arg:tt)*) => {{
        crate::SCORE_DEC.with(|score_dec| score_dec.set(score_dec.get() + 2));
        say!(::termcolor::Color::Red, true, b"BAD: ", $($arg)*);
    }};
    ($color:expr, $bold:expr, $prefix:literal, $($arg:tt)*) => {{
        use ::std::io::Write;
        use ::termcolor::WriteColor;
        let stdout = ::termcolor::StandardStream::stdout(
            $crate::COLOR_CHOICE.with(|color_choice| color_choice.get())
        );
        let mut stdout_lock = stdout.lock();
        stdout_lock.set_color(::termcolor::ColorSpec::new().set_fg(Some($color)).set_bold($bold)).unwrap();
        stdout_lock.write_all($prefix).unwrap();
        stdout_lock.reset().unwrap();
        writeln!(stdout_lock, $($arg)*).unwrap();
        stdout_lock.flush().unwrap();
    }};
}

/// [StructOpt](structopt) struct
#[derive(Parser, Debug)]
#[clap(about)]
struct Opt {
    /// Be quiet
    #[clap(short = 'q', long = "quiet")]
    quiet: bool,

    /// Be verbose (-v, -vv, -vvv, ...)
    #[clap(short = 'v', long = "verbose", parse(from_occurrences))]
    verbose: usize,

    /// Add timestamps to logging output
    #[clap(
        long = "timestamp",
        overrides_with("timestamp"),
        default_value("off"),
        possible_values(&["sec", "ms", "us", "ns", "none", "off"]),
    )]
    timestamp: stderrlog::Timestamp,

    /// Specify when to use colored output
    #[clap(
        long = "color",
        overrides_with("color-choice"),
        default_value("auto"),
        possible_values(&["always", "ansi", "auto", "never"]),
        parse(try_from_str = crate::utils::parse_color_choice),
    )]
    color_choice: ColorChoice,
}

/// The main function.
fn main() {
    #[cfg(feature = "color-backtrace")]
    color_backtrace::install();

    print!(concat!(
        "raudit  Copyright © 2021 rusty-snake\n",
        "This program comes with ABSOLUTELY NO WARRANTY.\n",
        "This is free software, and you are welcome to redistribute it\n",
        "and/or modify it under the terms of the GNU General Public License\n",
        "as published by the Free Software Foundation; either version 3 of\n",
        "the License, or (at your option) any later version.\n",
    ));

    let opt = if let Ok(args) = env::var("RAUDIT_ARGS") {
        Opt::from_iter(args.split_whitespace())
    } else {
        Opt::from_args()
    };

    StdErrLog::new()
        .verbosity(opt.verbose + 1) // error+warn as default
        .quiet(opt.quiet)
        .show_level(true)
        .timestamp(opt.timestamp)
        .color(opt.color_choice)
        .module(module_path!())
        .show_module_names(true)
        .init()
        .unwrap();

    COLOR_CHOICE.with(|color_choice| {
        color_choice.set(opt.color_choice);
    });

    io::stdin()
        .lock()
        .lines()
        .filter_map(|result| match result {
            Ok(line) => Some(line),
            Err(err) => {
                error!("An error occurred while processing the rules: {}", err);
                None
            }
        })
        .filter(|line| !(line.is_empty() || line.starts_with('#')))
        .inspect(|line| trace!("{}", line))
        .filter_map(|line| match Rule::from_str(&line) {
            Ok(rule) => Some(rule),
            Err(err) => {
                error!("{}", err);
                None
            }
        })
        .for_each(|rule| rule.check());

    let score_dec = SCORE_DEC.with(|score_dec| score_dec.get());
    let score_max = SCORE_MAX.with(|score_max| score_max.get());
    println!(
        "Your score: {} out of {}.",
        score_max - score_dec,
        score_max
    );
}

/// Whether we are in the real $HOME. (heuristic!)
///
/// This function checks a list of 32 files ATOW which are commonly found
/// in a home directory. If at least 5 are present, `true` is returned.
/// The files in the list and the minimum of present files can change at
/// any time.
///
/// Do not call this function direct, use the cached result in
/// [`struct@REAL_HOME`] instead.
fn real_home() -> bool {
    const PATHS: [&str; 32] = [
        "~/.bash_history",
        "~/.gitconfig",
        "~/.gnupg",
        "~/.lesshst",
        "~/.netrc",
        "~/.pki",
        "~/.ssh",
        "~/.var",
        "~/.viminfo",
        "~/.wget-hsts",
        "~/.cache/dconf",
        "~/.cache/flatpak",
        "~/.cache/gegl-0.4",
        "~/.cache/gnome-software",
        "~/.cache/gstreamer-1.0",
        "~/.cache/ibus",
        "~/.cache/mesa_shader_cache",
        "~/.cache/samba",
        "~/.cache/thumbnails",
        "~/.cache/tracke",
        "~/.cache/tracker3",
        "~/.config/autostart",
        "~/.config/enchant",
        "~/.local/bin",
        "~/.local/share/flatpak",
        "~/.local/share/gstreamer-1.0",
        "~/.local/share/gvfs-metadata",
        "~/.local/share/pki",
        "~/.local/share/recently-used.xbel",
        "~/.local/share/tracker",
        "~/.local/share/Trash",
        "~/.local/share/webkitgtk",
    ];

    let mut cnt = 0;
    for path in &PATHS {
        trace!("real_home: Checking `{}'", path);
        if access(&*expand_tilde(path), AccessFlags::F_OK).is_ok() {
            debug!("real_home: Found `{}'", path);
            cnt += 1;
            if cnt >= 5 {
                return true;
            }
        }
    }

    false
}
