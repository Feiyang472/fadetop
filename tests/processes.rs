//! Tests for `fadetop::processes::PythonProcess::display_name`.

use fadetop::processes::PythonProcess;

fn proc(cmdline: &str) -> PythonProcess {
    PythonProcess {
        pid: 1,
        cmdline: cmdline.to_string(),
    }
}

#[test]
fn display_name_returns_script_path() {
    let p = proc("python3 /home/user/scripts/web_server.py --port 8080");
    assert_eq!(p.display_name(), "/home/user/scripts/web_server.py");
}

#[test]
fn display_name_skips_dash_arg_and_falls_back_to_full_cmdline() {
    // `-m pytest` — the second token starts with '-', so the helper gives up
    // and returns the full cmdline. (Documents the current behavior — this is
    // arguably suboptimal but is what the code does today.)
    let p = proc("python3 -m pytest tests/");
    assert_eq!(p.display_name(), "python3 -m pytest tests/");
}

#[test]
fn display_name_skips_short_flag_arg() {
    let p = proc("python3 -c 'import time; time.sleep(3600)'");
    assert_eq!(p.display_name(), "python3 -c 'import time; time.sleep(3600)'");
}

#[test]
fn display_name_with_only_interpreter_returns_cmdline() {
    let p = proc("python3");
    assert_eq!(p.display_name(), "python3");
}

#[test]
fn display_name_handles_empty_cmdline() {
    let p = proc("");
    assert_eq!(p.display_name(), "");
}
