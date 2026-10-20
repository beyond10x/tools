//! The `process` toolset: `process.run`, one declared program started by an argv in one workspace
//! root, under the `tool-contract/1` contract in `contracts/process/v1alpha1`.
//!
//! This crate is harness's `crates/harness-tools/src/local.rs` process runner at harness 0.14.1
//! (`f109612e`) moved here: `INHERITED_ENV`, `MAX_RUN_OUTPUT_BYTES`, `MAX_RUN_SECONDS`, the
//! declared program set and `LocalOperations::inheriting`, `exec` (now [`Process::run`]),
//! `terminate`, `drain`, `read_ends`, `keep_both_ends`, `whole_prefix` and `whole_suffix`, with the
//! same reply fields, bounds and refusals, and its tests for both ends of the output, the omitted
//! count, character boundaries, the run bounded by what is left, the group kill, the program not
//! declared and the cleared environment. Added here: the caller's own `timeout_ms`, capped by the
//! contract's maximum and by what the run has left; typed input and output ([`types`]) whose
//! `schemars` schemas the conformance run checks against the contract; [`ProcessError`] in place
//! of harness's `Refused`, each variant named by one contract error; the
//! [`tool_contract::Toolset`] implementation; and the mutants the suite is checked against. Left
//! in harness: everything about files, `grep`, the catalogue and the substrate.
//!
//! # Where it differs from harness 0.14.1
//!
//! Each pinned by a scenario or a test. The run lasts until the program **and** both streams
//! have ended, under the bound: harness stopped watching the clock once the program ended and
//! joined the streams with no deadline, so a background descendant holding them kept the call
//! open past the bound and the reply said `timed_out: false`. At the bound the group is killed
//! even when the program itself has already ended. After the kill the run waits at most
//! half a second for the streams and the group, then answers with what was read so far, so a
//! process that left the group cannot hold the call open; the streams are read into shared ends
//! for that, never joined. A cut moves only for the character it broke: harness's `whole_prefix`
//! asked whether the whole head was UTF-8, so one stray byte anywhere left the cut through a
//! character and the count short, and its `whole_suffix` skipped any run of continuation bytes,
//! dropping the stream's own bytes into the count. The root is checked before a start, and again
//! when a start fails, by resolving `<root>/.`, so a removed root or one that can no longer be
//! entered is `process.WorkspaceUnavailable` and not the program's failure. The process group is
//! killed when waiting on a started program fails, where harness returned and left it running.
//!
//! # The boundary, and what it is not
//!
//! The declared set, the cleared environment and the bound are this crate's own: nothing confines
//! the started program. A declared shell can do whatever the user running it can, inside the
//! workspace and outside it. It is the right runner for an operator's own tree on their own
//! machine; a run that wants the effects confined starts it under a confined root.

mod error;
mod output;
mod toolset;
pub mod types;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use output::Drain;

pub use error::ProcessError;
pub use toolset::{CONTRACT_YAML, contract};
pub use types::{RunInput, RunOutput};

/// How much of each of a child's streams a run answers with: the first half and the last half.
pub const MAX_RUN_OUTPUT_BYTES: usize = 64 * 1024;

/// How long a run may hold the call open before its process group is killed, and the bound a call
/// gets when it names none.
///
/// A bound rather than none: `wait` on a child that never exits stops the caller with no record
/// of why. Ten minutes is longer than any suite this has been pointed at and shorter than a night.
pub const MAX_RUN_SECONDS: u64 = 600;

/// The environment variables a declared program is started with, unless a caller names more.
///
/// An allow list rather than a deny list, because the parent process's environment is where the
/// caller's credentials live and the child's arguments came from somebody else. Naming what may
/// cross keeps a token nobody thought about out of a program somebody else chose.
///
/// The toolchain names are here because without them `cargo` under a relocated rustup started
/// with `HOME` and `PATH` alone, looked in `~/.rustup`, and failed with *no default toolchain
/// configured*, a build failure that read as the workspace's. None of them carries a secret: each
/// is a path. A proxy URL can carry one, so `HTTPS_PROXY` and its kin are not here; a caller that
/// needs them names them with [`Process::inheriting`].
pub const INHERITED_ENV: &[&str] = &[
    "PATH",
    "HOME",
    "LANG",
    "LC_ALL",
    "TERM",
    "TMPDIR",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "CARGO_TARGET_DIR",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
];

/// One property switched off, for the mutant check the conformance suite is held to.
///
/// Only a mutant build (`--features mutants`, or the crate's own tests) can set one, through
/// `Process::mutated`; outside it every [`Process`] is intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutation {
    /// The declared set is not consulted: any `argv[0]` is started.
    NoProgramCheck,
    /// The environment is not cleared: the child inherits all of it.
    InheritAll,
    /// The deadline kills nothing: the run waits for the program's own end.
    NoKill,
    /// The deadline kills the direct child only, not its process group.
    NoGroupKill,
    /// The deadline kills the group only while the program itself is still running: a
    /// descendant left by a program that already ended is left alive.
    KillOnlyALiveProgram,
    /// Both streams are kept whole, however long.
    NoOutputCap,
    /// A caller's `timeout_ms` above the maximum is not capped.
    NoTimeoutCap,
}

impl Mutation {
    /// Every mutation, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::NoProgramCheck,
        Self::InheritAll,
        Self::NoKill,
        Self::NoGroupKill,
        Self::KillOnlyALiveProgram,
        Self::NoOutputCap,
        Self::NoTimeoutCap,
    ];

    /// The mutation's name in reports.
    pub fn name(self) -> &'static str {
        match self {
            Self::NoProgramCheck => "no-program-check",
            Self::InheritAll => "inherit-all",
            Self::NoKill => "no-kill",
            Self::NoGroupKill => "no-group-kill",
            Self::KillOnlyALiveProgram => "kill-only-a-live-program",
            Self::NoOutputCap => "no-output-cap",
            Self::NoTimeoutCap => "no-timeout-cap",
        }
    }
}

/// Runs declared programs in one directory.
#[derive(Debug, Clone)]
pub struct Process {
    root: PathBuf,
    /// What `run` will start, by exact `argv[0]`; may be empty, which starts nothing.
    programs: Vec<String>,
    /// Environment variables a declared program is started with, beyond [`INHERITED_ENV`].
    inherited: Vec<String>,
    #[cfg(any(test, feature = "mutants"))]
    mutation: Option<Mutation>,
}

impl Process {
    /// Opens `root` as the directory programs run in, with `programs` as the declared set.
    ///
    /// # Errors
    ///
    /// `root` cannot be canonicalised, or is not a directory.
    pub fn new(root: impl AsRef<Path>, programs: Vec<String>) -> Result<Self, ProcessError> {
        Ok(Self {
            root: Self::open(root)?,
            programs,
            inherited: Vec::new(),
            #[cfg(any(test, feature = "mutants"))]
            mutation: None,
        })
    }

    /// The same runner with one property switched off, for the mutant check.
    ///
    /// # Errors
    ///
    /// As [`Process::new`].
    #[cfg(any(test, feature = "mutants"))]
    pub fn mutated(
        root: impl AsRef<Path>,
        programs: Vec<String>,
        mutation: Mutation,
    ) -> Result<Self, ProcessError> {
        Ok(Self {
            mutation: Some(mutation),
            ..Self::new(root, programs)?
        })
    }

    /// Names more of this process's environment that a declared program may see.
    ///
    /// The default list is what a toolchain needs and nothing that carries a credential. A run
    /// that needs a proxy, or a variable its build reads, names it here: by name, at the call
    /// site, where a reader can see what was let through.
    #[must_use]
    pub fn inheriting(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.inherited.extend(names.into_iter().map(Into::into));
        self
    }

    /// The canonical workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The declared program set, in the order it was declared.
    pub fn programs(&self) -> &[String] {
        &self.programs
    }

    fn open(root: impl AsRef<Path>) -> Result<PathBuf, ProcessError> {
        let given = root.as_ref().display().to_string();
        let root = root
            .as_ref()
            .canonicalize()
            .map_err(|source| ProcessError::Workspace {
                path: given.clone(),
                source,
            })?;
        if !root.is_dir() {
            return Err(ProcessError::WorkspaceNotADirectory { path: given });
        }
        Ok(root)
    }

    #[cfg(any(test, feature = "mutants"))]
    fn mutated_by(&self, mutation: Mutation) -> bool {
        self.mutation == Some(mutation)
    }

    #[cfg(not(any(test, feature = "mutants")))]
    #[allow(clippy::unused_self)]
    fn mutated_by(&self, _mutation: Mutation) -> bool {
        false
    }

    /// Runs one declared program and answers what it did.
    ///
    /// **An argv, never a command line.** Nothing here builds a string a shell would take apart.
    /// The program runs in the workspace root with no standard input and an environment cleared
    /// down to [`INHERITED_ENV`] and the names given to [`Process::inheriting`], in a process
    /// group of its own.
    ///
    /// `remaining` is how much of the caller's own wall-clock budget is left. A program is the
    /// one thing that outlives the call that started it by minutes, and a caller's deadline check
    /// between calls cannot reach into this one, so the bound is set here: the smallest of the
    /// input's `timeout_ms` (default [`MAX_RUN_SECONDS`]), [`MAX_RUN_SECONDS`] and `remaining`.
    /// The run lasts until the program and both of its streams have ended. When the bound comes
    /// first, the whole process group is killed, whether or not the program has already ended;
    /// the run waits at most [`SETTLE`] for the streams and the group, then answers with what was
    /// read, `timed_out: true`. A process that left the group is not killed, only no longer
    /// waited for.
    ///
    /// # Errors
    ///
    /// `argv` is empty; `argv[0]` is not declared, compared as text; the workspace root can no
    /// longer be entered; the program could not be started; or the system refused to report on
    /// the started program, whose group is then killed. A program that exits non-zero is not an
    /// error: it is a [`RunOutput`] with its `exit`.
    pub fn run(
        &self,
        input: &RunInput,
        remaining: Option<Duration>,
    ) -> Result<RunOutput, ProcessError> {
        let argv = &input.argv;
        let Some(program) = argv.first() else {
            return Err(ProcessError::EmptyArgv);
        };
        if !self.mutated_by(Mutation::NoProgramCheck)
            && !self.programs.iter().any(|declared| declared == program)
        {
            // Named, not only worded: the sentence is what a model reads; the fields beside it
            // are what makes *the run would not start this* countable on the record.
            return Err(ProcessError::ProgramNotDeclared {
                program: program.clone(),
                declared: self.programs.clone(),
            });
        }
        self.enterable()?;

        let mut child = match self.command(program, &argv[1..]).spawn() {
            Ok(child) => child,
            Err(source) => {
                // A start fails the same way for a root that cannot be entered as for a program
                // that cannot be found, so the root is asked again before the program is blamed.
                self.enterable()?;
                return Err(ProcessError::SpawnFailed {
                    program: program.clone(),
                    source,
                });
            }
        };
        let wait_failed = |source| ProcessError::WaitFailed {
            program: program.clone(),
            source,
        };

        let half = if self.mutated_by(Mutation::NoOutputCap) {
            usize::MAX
        } else {
            MAX_RUN_OUTPUT_BYTES / 2
        };
        let out = Drain::start(child.stdout.take(), half);
        let err = Drain::start(child.stderr.take(), half);

        // The run is over when the program has ended **and** both streams have: a descendant in
        // the group that inherited a pipe is still the run. Harness stopped watching the clock
        // once the program ended and then joined the streams with no deadline, so a background
        // `sleep` holding them kept the call open past the bound and the reply said it had not
        // timed out.
        let bound = self.bound(input.timeout_ms, remaining);
        let deadline = Instant::now().checked_add(bound);
        let mut ended = None;
        let timed_out = loop {
            if ended.is_none() {
                match child.try_wait() {
                    Ok(status) => ended = status,
                    Err(source) => {
                        // Harness returned here and left the program running with nobody to stop
                        // it.
                        terminate(&mut child);
                        return Err(wait_failed(source));
                    }
                }
            }
            if ended.is_some() && out.finished() && err.finished() {
                break false;
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                break true;
            }
            thread::sleep(POLL);
        };
        if timed_out && (ended.is_none() || !self.mutated_by(Mutation::KillOnlyALiveProgram)) {
            // The whole group, whether or not the program itself has already ended.
            self.stop(&mut child);
        }
        let status = match ended {
            Some(status) => status,
            None => child.wait().map_err(wait_failed)?,
        };
        if timed_out {
            self.settle(&child, &out, &err);
        }

        let stdout = out.drained();
        let stderr = err.drained();
        let omitted = stdout.omitted + stderr.omitted;
        Ok(RunOutput {
            argv: argv.clone(),
            exit: status.code(),
            stdout: stdout.text,
            stderr: stderr.text,
            truncated: omitted > 0,
            // Across both streams. Each stream's own marker names its share, in the place the
            // bytes were dropped from, so a reader never has to work out which one was cut.
            omitted_bytes: omitted,
            // Named, because a killed process's exit code says nothing about the task.
            timed_out,
            // And how long it was given, so a kill at the deadline is not read as the program's
            // own slowness.
            timeout_ms: u64::try_from(bound.as_millis()).unwrap_or(u64::MAX),
        })
    }

    /// `program` with `args`, set up to start in the root with no standard input, piped output,
    /// the allow-listed environment and a process group of its own.
    fn command(&self, program: &str, args: &[String]) -> Command {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(&self.root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if !self.mutated_by(Mutation::InheritAll) {
            // Cleared, then filled from the allow-list alone. A child started here inherits the
            // whole of this process's environment otherwise, which is how a credential held for
            // the caller reaches a program somebody else picked the arguments for.
            command.env_clear();
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            // A private process group makes the timeout cover descendants that inherited the
            // output pipes, not only the shell directly returned by `spawn`.
            command.process_group(0);
        }
        // `var_os`, not `var`: a value that is not UTF-8 is still the value, and `var` would drop
        // it without a word.
        for name in INHERITED_ENV
            .iter()
            .copied()
            .chain(self.inherited.iter().map(String::as_str))
        {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command
    }

    /// The smallest of the caller's own bound (or the default), the ceiling and what the run has
    /// left.
    fn bound(&self, asked_ms: Option<u64>, remaining: Option<Duration>) -> Duration {
        let ceiling_ms = MAX_RUN_SECONDS * 1000;
        let asked_ms = asked_ms.unwrap_or(ceiling_ms);
        let capped_ms = if self.mutated_by(Mutation::NoTimeoutCap) {
            asked_ms
        } else {
            asked_ms.min(ceiling_ms)
        };
        let bound = Duration::from_millis(capped_ms);
        remaining.map_or(bound, |left| left.min(bound))
    }

    /// Whether the root can still be entered: `<root>/.` resolves only through a directory this
    /// process may search, which is what starting a program in it needs.
    ///
    /// `is_dir` alone answered yes for a directory whose search permission was taken away after
    /// it opened, and the start then failed as the program's.
    fn enterable(&self) -> Result<(), ProcessError> {
        fs::metadata(self.root.join("."))
            .map(drop)
            .map_err(|source| ProcessError::Workspace {
                path: self.root.display().to_string(),
                source,
            })
    }

    /// Stops the program at the bound: its whole process group.
    fn stop(&self, child: &mut Child) {
        if self.mutated_by(Mutation::NoKill) {
            return;
        }
        if self.mutated_by(Mutation::NoGroupKill) {
            let _ = child.kill();
            return;
        }
        terminate(child);
    }

    /// After the kill at the bound: waits, for at most [`SETTLE`], until both streams have ended
    /// and the group is gone, so the reply carries what the group wrote before it died and no
    /// member of it outlives the call.
    ///
    /// Bounded, because a process that left the group (`setsid`, a shell's job control) is not
    /// killed and can hold a stream open for as long as it likes; past [`SETTLE`] the run stops
    /// waiting for it and answers with what was read so far.
    fn settle(&self, child: &Child, out: &Drain, err: &Drain) {
        let limit = if self.mutated_by(Mutation::NoKill) {
            None
        } else {
            Instant::now().checked_add(SETTLE)
        };
        let settled = || out.finished() && err.finished() && (limit.is_none() || group_gone(child));
        while !settled() && limit.is_none_or(|limit| Instant::now() < limit) {
            thread::sleep(SETTLE_POLL);
        }
    }
}

/// How often a running program is asked whether it has ended.
const POLL: Duration = Duration::from_millis(25);

/// How long a run killed at its bound waits for its streams to end and its group to be gone
/// before it answers anyway.
const SETTLE: Duration = Duration::from_millis(500);

/// How often a killed run looks again while it settles.
const SETTLE_POLL: Duration = Duration::from_millis(5);

/// Kills the child's whole process group, then the child itself. The group is named by the
/// child's pid, which stays the group's id while any member is alive, so this reaches a
/// descendant even after the child itself has ended and been reaped.
#[cfg(unix)]
fn terminate(child: &mut Child) {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;

    if let Ok(pid) = i32::try_from(child.id()) {
        let _ = killpg(Pid::from_raw(pid), Signal::SIGKILL);
    }
    let _ = child.kill();
}

/// Kills the child; with no process groups there is nothing more to reach.
#[cfg(not(unix))]
fn terminate(child: &mut Child) {
    let _ = child.kill();
}

/// Whether no process is left in the child's group, a dead one not yet reaped included.
#[cfg(unix)]
fn group_gone(child: &Child) -> bool {
    use nix::errno::Errno;
    use nix::sys::signal::killpg;
    use nix::unistd::Pid;

    i32::try_from(child.id()).map_or(true, |pid| {
        killpg(Pid::from_raw(pid), None) == Err(Errno::ESRCH)
    })
}

/// With no process groups, the child is the run.
#[cfg(not(unix))]
fn group_gone(_child: &Child) -> bool {
    true
}
