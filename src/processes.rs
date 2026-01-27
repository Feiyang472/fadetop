use remoteprocess::Pid;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PythonProcess {
    pub pid: Pid,
    pub cmdline: String,
}

/// Trait for discovering Python processes
pub trait ProcessDiscovery {
    fn discover() -> Vec<PythonProcess>;
}

/// Real process discovery using the system's /proc filesystem
pub struct SystemProcessDiscovery;

impl ProcessDiscovery for SystemProcessDiscovery {
    fn discover() -> Vec<PythonProcess> {
        discover_python_processes()
    }
}

impl PythonProcess {
    /// Returns a short display name (script name or interpreter)
    pub fn display_name(&self) -> &str {
        // Try to extract the first meaningful argument (script name)
        self.cmdline
            .split_whitespace()
            .nth(1)
            .and_then(|arg| {
                if arg.starts_with('-') {
                    None
                } else {
                    Some(arg)
                }
            })
            .unwrap_or(&self.cmdline)
    }
}

/// Discovers Python processes on the system
fn discover_python_processes() -> Vec<PythonProcess> {
    #[cfg(target_os = "linux")]
    {
        discover_linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "linux")]
fn discover_linux() -> Vec<PythonProcess> {
    let mut processes = Vec::new();
    let current_pid = std::process::id() as Pid;

    let Ok(proc_entries) = fs::read_dir("/proc") else {
        return processes;
    };

    for entry in proc_entries.flatten() {
        let path = entry.path();

        // Check if this is a PID directory
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Ok(pid) = name.parse::<Pid>() else {
            continue;
        };

        // Skip self
        if pid == current_pid {
            continue;
        }

        // Check if it's a Python process
        if !is_python_process(&path) {
            continue;
        }

        // Read command line
        let cmdline = read_cmdline(&path);
        if cmdline.is_empty() {
            continue;
        }

        processes.push(PythonProcess { pid, cmdline });
    }

    // Sort by PID for stable ordering
    processes.sort_by_key(|p| p.pid);
    processes
}

#[cfg(target_os = "linux")]
fn is_python_process(proc_path: &Path) -> bool {
    let exe_path = proc_path.join("exe");

    // Read the symlink target
    if let Ok(target) = fs::read_link(&exe_path) {
        let target_str = target.to_string_lossy();
        // Check if executable is python
        if target_str.contains("python") {
            return true;
        }
    }

    // Fallback: check /proc/[pid]/comm
    let comm_path = proc_path.join("comm");
    if let Ok(comm) = fs::read_to_string(&comm_path) {
        let comm = comm.trim();
        if comm.starts_with("python") {
            return true;
        }
    }

    false
}

#[cfg(target_os = "linux")]
fn read_cmdline(proc_path: &Path) -> String {
    let cmdline_path = proc_path.join("cmdline");

    fs::read(&cmdline_path)
        .map(|bytes| {
            // cmdline is null-separated
            bytes
                .split(|&b| b == 0)
                .filter(|s| !s.is_empty())
                .map(|s| String::from_utf8_lossy(s).into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}
