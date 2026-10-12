//! Tests for `fadetop::processes`.

use fadetop::processes::{PythonProcess, display_name, is_python_executable};

#[test]
fn display_name_returns_script_path() {
    let args = [
        "python3",
        "/home/user/scripts/web_server.py",
        "--port",
        "8080",
    ];
    assert_eq!(display_name(&args), "/home/user/scripts/web_server.py");
}

#[test]
fn display_name_shows_module() {
    assert_eq!(
        display_name(&["python3", "-m", "pytest", "tests/"]),
        "-m pytest"
    );
    assert_eq!(display_name(&["python3", "-mpytest"]), "-m pytest");
}

#[test]
fn display_name_shows_inline_code() {
    let args = ["python3", "-c", "import time; time.sleep(3600)"];
    assert_eq!(display_name(&args), "-c import time; time.sleep(3600)");
}

#[test]
fn display_name_skips_interpreter_options() {
    let args = [
        "python3", "-u", "-X", "dev", "-W", "error", "train.py", "--lr", "3",
    ];
    assert_eq!(display_name(&args), "train.py");
}

#[test]
fn display_name_with_only_interpreter_returns_cmdline() {
    assert_eq!(display_name(&["python3"]), "python3");
    assert_eq!(display_name(&["python3", "-i"]), "python3 -i");
}

#[test]
fn display_name_handles_empty_cmdline() {
    assert_eq!(display_name::<&str>(&[]), "");
}

#[test]
fn recognises_python_executables() {
    for name in [
        "python",
        "python3",
        "python3.12",
        "python3.13t",
        "python.exe",
        "pythonw.exe",
        "Python3.11.EXE",
    ] {
        assert!(is_python_executable(name), "{name}");
    }
    for name in [
        "pythonista",
        "python-lsp-server",
        "ipython",
        "pip",
        "",
        "py",
    ] {
        assert!(!is_python_executable(name), "{name}");
    }
}

#[test]
fn filter_matches_pid_name_and_cmdline() {
    let p = PythonProcess::new(4242, &["python3", "-m", "http.server", "8000"]);
    assert!(p.matches("4242"));
    assert!(p.matches("HTTP"));
    assert!(p.matches("8000"));
    assert!(!p.matches("pytest"));
}
