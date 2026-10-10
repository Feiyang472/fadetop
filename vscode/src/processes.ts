import { execFile } from "child_process";
import * as path from "path";

export interface PythonProcess {
    pid: number;
    cmdline: string;
}

function run(cmd: string, args: string[]): Promise<string> {
    return new Promise((resolve, reject) =>
        execFile(cmd, args, { maxBuffer: 16 * 1024 * 1024, windowsHide: true }, (err, stdout) =>
            err ? reject(err) : resolve(stdout),
        ),
    );
}

function isPython(cmdline: string): boolean {
    const exe = cmdline.split(/\s+/)[0] ?? "";
    return /^python/i.test(path.basename(exe.replace(/\\/g, "/")));
}

/** Lists running Python processes, sorted by PID (same as the TUI dashboard). */
export async function discoverPythonProcesses(): Promise<PythonProcess[]> {
    let rows: PythonProcess[];
    if (process.platform === "win32") {
        const script =
            "Get-CimInstance Win32_Process -Filter \"Name like 'python%'\" | " +
            "ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }";
        const out = await run("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script]);
        rows = parse(out).map((p) => ({ ...p, cmdline: p.cmdline.replace(/^"([^"]*)"/, "$1") }));
    } else {
        rows = parse(await run("ps", ["-axww", "-o", "pid=", "-o", "args="]));
    }
    return rows
        .filter((p) => p.pid !== process.pid && isPython(p.cmdline))
        .sort((a, b) => a.pid - b.pid);
}

function parse(out: string): PythonProcess[] {
    return out
        .split(/\r?\n/)
        .map((line) => /^\s*(\d+)\s+(.+?)\s*$/.exec(line))
        .filter((m): m is RegExpExecArray => m !== null)
        .map((m) => ({ pid: Number(m[1]), cmdline: m[2] }));
}

/** Same rule as `PythonProcess::display_name` in src/processes.rs. */
export function displayName(p: PythonProcess): string {
    const arg = p.cmdline.split(/\s+/).filter(Boolean)[1];
    return arg && !arg.startsWith("-") ? arg : p.cmdline;
}
