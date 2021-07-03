# raudit

[![GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-darkred?logo=gnu&logoColor=darkred&labelColor=silver&style=for-the-badge)](COPYING)
[![maintenance-status: as-is](https://img.shields.io/badge/maintenance--status-as--is-yellow?labelColor=silver&style=for-the-badge)](https://gist.github.com/rusty-snake/574a91f1df9f97ec77ca308d6d731e29)

A configurable audit program for firejail-sandboxes with metrics.

faudit was the default audit program for [firejail](https://firejail.wordpress.com/)
(in firejail 0.9.66 it was replaced by jailcheck). It is a good way to get an
impressions of gaps in a firejail profile. However, it can not be configured,
all check are hardcoded. raudit tries to fix this.

### Project history

raudit started as a configurable alternative to faudit to be more flexiable.
Nowadays it is a rootless alternative to jailcheck.

## Build and Install

Install [Rust](https://www.rust-lang.org/tools/install) and get the source code
(e.g. `git clone https://github.com/rusty-snake/raudit.git && cd raudit`).
Afterwards you can build raudit using `cargo build --release --features=color-backtrace`,
strip the binary if you want (`strip target/release/raudit`) and install it by

```bash
PREFIX=/usr/local
sudo install -Dm0755 target/release/raudit $PREFIX/libexec/raudit
sudo install -Dm0644 -t $PREFIX/share/raudit share/*.rules
```

Optionally you can build and install the man-page too.

```bash
make -C man man
sudo install -Dm0644 man/raudit.7.gz $PREFIX/share/man/man7/raudit.7.gz
```

## Example

```
$ firejail --profile=firefox /proc/self/fd/3 </usr/local/share/raudit/default.rules 3</usr/local/libexec/raudit
Reading profile /etc/firejail/firefox.profile
[...]
GREAT: The capability bounding set is empty.
GREAT: no_new_privs is set, the sandbox can not acquire new privileges using execve.
Check write access to "Initialization files that allow arbitrary command execution" from disable-common.inc
GOOD: The sandbox cannot write to /home/rusty-snake/.caffrc.
GOOD: The sandbox cannot write to /home/rusty-snake/.cargo/env.
GOOD: The sandbox cannot write to /home/rusty-snake/.dotfiles.
[...]
GOOD: The sandbox cannot write to /home/rusty-snake/_vimrc.
GOOD: The sandbox cannot write to /home/rusty-snake/dotfiles.
Check read access to "top secret" from disable-common.inc
GOOD: The sandbox cannot read /home/rusty-snake/.Private because it does not exist.
GOOD: The sandbox cannot read /home/rusty-snake/.caff because it does not exist.
GOOD: The sandbox cannot read /home/rusty-snake/.cargo/credentials because it does not exist.
[...]
GOOD: The sandbox cannot read /home/aurora/.nyx because it does not exist.
UGLY: The sandbox can read /home/rusty-snake/.pki.
UGLY: The sandbox can read /home/rusty-snake/.local/share/pki.
GOOD: The sandbox cannot read /home/rusty-snake/.smbcredentials because it does not exist.
GOOD: The sandbox cannot read /home/rusty-snake/.ssh because it does not exist.
[...]
GOOD: The sandbox cannot read /etc/shadow-.
GOOD: The sandbox cannot read /etc/ssh.
[...]
Your score: 77 out of 79.

Parent is shutting down, bye...
```
