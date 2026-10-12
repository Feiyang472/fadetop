use remoteprocess::Pid;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// A running Python process, as listed by every frontend (the TUI dashboard
/// and `fadetop list-processes`, which the VS Code extension consumes).
#[derive(Debug, Clone)]
pub struct PythonProcess {
    pub pid: Pid,
    /// Short label: the script, `-m module` or `-c code`.
    pub name: String,
    pub cmdline: String,
    /// Resident memory in bytes.
    pub memory: u64,
    /// Start time in seconds since the Unix epoch (0 if unknown).
    pub start_time: u64,
}

/// Trait for discovering Python processes
pub trait ProcessDiscovery {
    fn discover() -> Vec<PythonProcess>;
}

/// Cross-platform process discovery backed by `sysinfo`
pub struct SystemProcessDiscovery;

impl ProcessDiscovery for SystemProcessDiscovery {
    fn discover() -> Vec<PythonProcess> {
        discover_python_processes()
    }
}

impl PythonProcess {
    pub fn new<S: AsRef<str>>(pid: Pid, args: &[S]) -> Self {
        let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
        Self {
            pid,
            name: display_name(&args),
            cmdline: args.join(" "),
            memory: 0,
            start_time: 0,
        }
    }

    /// Case-insensitive match against the PID, name and command line.
    pub fn matches(&self, filter: &str) -> bool {
        let filter = filter.to_lowercase();
        self.pid.to_string().contains(&filter)
            || self.name.to_lowercase().contains(&filter)
            || self.cmdline.to_lowercase().contains(&filter)
    }

    /// Human-readable resource summary, e.g. `"45.3 MB, up 5m"`.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.memory > 0 {
            parts.push(format_bytes(self.memory));
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        if self.start_time > 0 && now >= self.start_time {
            parts.push(format!("up {}", format_age(now - self.start_time)));
        }
        parts.join(", ")
    }
}

impl Serialize for PythonProcess {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("PythonProcess", 6)?;
        s.serialize_field("pid", &self.pid)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("cmdline", &self.cmdline)?;
        s.serialize_field("memory", &self.memory)?;
        s.serialize_field("start_time", &self.start_time)?;
        s.serialize_field("summary", &self.summary())?;
        s.end()
    }
}

/// Short label for a Python command line: the script path, `-m module` or
/// `-c code`, skipping interpreter options. Falls back to the whole command.
pub fn display_name<S: AsRef<str>>(args: &[S]) -> String {
    let mut rest = args.iter().map(AsRef::as_ref).skip(1);
    while let Some(arg) = rest.next() {
        match arg {
            "-m" | "-c" => {
                return match rest.next() {
                    Some(value) => format!("{arg} {value}"),
                    None => arg.to_string(),
                };
            }
            // Options that consume the next argument
            "-W" | "-X" | "--check-hash-based-pycs" => {
                rest.next();
            }
            "-" => return "<stdin>".to_string(),
            _ if arg.starts_with("-m") || arg.starts_with("-c") => {
                return format!("{} {}", &arg[..2], &arg[2..]);
            }
            _ if arg.starts_with('-') => {}
            _ => return arg.to_string(),
        }
    }
    args.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" ")
}

/// Whether an executable name looks like a CPython interpreter:
/// `python`, `python3`, `python3.12`, `python3.13t`, `pythonw.exe`, ...
pub fn is_python_executable(name: &str) -> bool {
    let name = name.to_lowercase();
    let name = name.strip_suffix(".exe").unwrap_or(&name);
    let Some(rest) = name.strip_prefix("python") else {
        return false;
    };
    let rest = rest.strip_prefix('w').unwrap_or(rest);
    let version = rest.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    rest.len() - version.len() <= 1 && version.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// Discovers Python processes on the system, sorted by PID
fn discover_python_processes() -> Vec<PythonProcess> {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet)
            .with_memory(),
    );
    let current_pid = sysinfo::get_current_pid().ok();

    let mut processes: Vec<PythonProcess> = system
        .processes()
        .values()
        .filter(|p| Some(p.pid()) != current_pid && p.thread_kind().is_none())
        .filter_map(|p| {
            let args: Vec<String> = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            // The executable path is unreadable for other users' processes on
            // some platforms, so also check the process name and argv[0].
            let is_python = p.exe().is_some_and(|e| is_python_executable(file_name(e)))
                || is_python_executable(&p.name().to_string_lossy())
                || args
                    .first()
                    .is_some_and(|a| is_python_executable(file_name(Path::new(a))));
            if !is_python {
                return None;
            }
            // Command lines can be hidden too; fall back to the executable.
            let args = if args.is_empty() {
                let exe = p.exe().map_or_else(
                    || p.name().to_string_lossy().into_owned(),
                    |e| e.display().to_string(),
                );
                vec![exe]
            } else {
                args
            };
            Some(PythonProcess {
                memory: p.memory(),
                start_time: p.start_time(),
                ..PythonProcess::new(p.pid().as_u32() as Pid, &args)
            })
        })
        .collect();

    processes.sort_by_key(|p| p.pid);
    processes
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn format_age(secs: u64) -> String {
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86400 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86400),
    }
}
