import { execFile } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

export function activate(context: vscode.ExtensionContext) {
    context.subscriptions.push(
        vscode.commands.registerCommand("fadetop.openDashboard", async () => {
            const binary = resolveBinary(context);
            if (binary) {
                await launch(binary, [], "FadeTop");
            }
        }),
        vscode.commands.registerCommand("fadetop.attach", async (pid?: number) => {
            const binary = resolveBinary(context);
            if (!binary) {
                return;
            }
            pid ??= await pickProcess(binary);
            if (pid !== undefined) {
                await launch(binary, [String(pid)], `FadeTop: ${pid}`);
            }
        }),
    );
}

export function deactivate() {}

function bundledBinary(context: vscode.ExtensionContext): string {
    const exe = process.platform === "win32" ? "fadetop.exe" : "fadetop";
    return context.asAbsolutePath(path.join("bin", exe));
}

function resolveBinary(context: vscode.ExtensionContext): string | undefined {
    const configured = vscode.workspace.getConfiguration("fadetop").get<string | null>("binaryPath");
    const binary = configured || bundledBinary(context);
    if (!fs.existsSync(binary)) {
        vscode.window.showErrorMessage(
            configured
                ? `FadeTop binary not found at ${binary} (fadetop.binaryPath).`
                : "This FadeTop build has no bundled binary for your platform. Set fadetop.binaryPath to a fadetop executable.",
        );
        return undefined;
    }
    if (process.platform !== "win32") {
        // VSIX extraction does not always preserve the executable bit.
        try {
            fs.accessSync(binary, fs.constants.X_OK);
        } catch {
            fs.chmodSync(binary, 0o755);
        }
    }
    return binary;
}

function configEnv(): Record<string, string> {
    const config = vscode.workspace.getConfiguration("fadetop");
    const env: Record<string, string> = {};
    const samplingRate = config.get<number | null>("samplingRate");
    if (samplingRate) {
        env.FADETOP_SAMPLING_RATE = String(samplingRate);
    }
    const windowWidth = config.get<string | null>("windowWidth");
    if (windowWidth) {
        env.FADETOP_WINDOW_WIDTH = windowWidth;
    }
    const configFile = config.get<string | null>("configFile");
    if (configFile) {
        env.FADETOP_CONFIG = configFile;
    }
    return env;
}

/** On Linux, attaching to a non-child process needs ptrace_scope 0 or root. */
async function checkPtrace(useSudo: boolean): Promise<"ok" | "sudo" | "cancel"> {
    if (process.platform !== "linux" || useSudo || process.getuid?.() === 0) {
        return "ok";
    }
    let scope: string;
    try {
        scope = fs.readFileSync("/proc/sys/kernel/yama/ptrace_scope", "utf8").trim();
    } catch {
        return "ok";
    }
    if (scope === "0") {
        return "ok";
    }
    const choice = await vscode.window.showWarningMessage(
        `ptrace is restricted (kernel.yama.ptrace_scope = ${scope}), so FadeTop will likely fail to attach without root.`,
        "Run with sudo",
        "Continue anyway",
    );
    if (choice === "Run with sudo") {
        return "sudo";
    }
    return choice === "Continue anyway" ? "ok" : "cancel";
}

const EXIT_PROMPT = "fadetop exited with code %s, press Enter to close";
const HOLD_ON_FAILURE =
    '"$@"; code=$?; if [ "$code" -ne 0 ]; then ' +
    `printf "\\n[${EXIT_PROMPT}]" "$code"; read -r _; fi`;

async function launch(binary: string, args: string[], name: string) {
    const config = vscode.workspace.getConfiguration("fadetop");
    let useSudo = config.get<boolean>("useSudo", false) && process.platform !== "win32";
    const ptrace = await checkPtrace(useSudo);
    if (ptrace === "cancel") {
        return;
    }
    useSudo ||= ptrace === "sudo";

    // Keep the terminal open on failure so the error can be read.
    const env = configEnv();
    let shellPath: string;
    let shellArgs: string[];
    if (process.platform === "win32") {
        const quoted = `'${binary.replace(/'/g, "''")}'`;
        shellPath = "powershell.exe";
        shellArgs = [
            "-NoProfile",
            "-Command",
            `& ${quoted} ${args.join(" ")}; if ($LASTEXITCODE) { ` +
                `Read-Host "\`n[${EXIT_PROMPT.replace("%s", "$LASTEXITCODE")}]" }`,
        ];
    } else {
        let command = [binary, ...args];
        if (useSudo) {
            // sudo resets the environment, so pass settings through `env`.
            command = ["sudo", "env", ...Object.entries(env).map(([k, v]) => `${k}=${v}`), ...command];
        }
        shellPath = "/bin/sh";
        shellArgs = ["-c", HOLD_ON_FAILURE, "fadetop", ...command];
    }

    const terminal = vscode.window.createTerminal({
        name,
        shellPath,
        shellArgs,
        env,
        cwd: vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
        iconPath: new vscode.ThemeIcon("pulse"),
        location:
            config.get<string>("terminalLocation") === "panel"
                ? vscode.TerminalLocation.Panel
                : vscode.TerminalLocation.Editor,
    });
    terminal.show();
}

/** Shape of `fadetop list-processes --json` (src/processes.rs). */
interface PythonProcess {
    pid: number;
    name: string;
    cmdline: string;
    summary: string;
}

function listProcesses(binary: string): Promise<PythonProcess[]> {
    return new Promise((resolve, reject) =>
        execFile(binary, ["list-processes", "--json"], { windowsHide: true }, (err, stdout, stderr) => {
            if (err) {
                reject(new Error(stderr.trim() || err.message));
                return;
            }
            try {
                resolve(JSON.parse(stdout));
            } catch (parseErr) {
                reject(parseErr);
            }
        }),
    );
}

interface ProcessItem extends vscode.QuickPickItem {
    pid: number;
}

function toItem(p: PythonProcess): ProcessItem {
    return {
        pid: p.pid,
        label: `$(pulse) ${p.name}`,
        description: [`PID ${p.pid}`, p.summary].filter(Boolean).join(" · "),
        detail: p.name === p.cmdline ? undefined : p.cmdline,
    };
}

/** Quick-pick version of the TUI dashboard's process selection list. */
function pickProcess(binary: string): Promise<number | undefined> {
    return new Promise((resolve) => {
        const qp = vscode.window.createQuickPick<ProcessItem>();
        const refreshButton: vscode.QuickInputButton = {
            iconPath: new vscode.ThemeIcon("refresh"),
            tooltip: "Refresh",
        };
        qp.buttons = [refreshButton];
        qp.placeholder = "Filter by name, command line or PID, or type a PID";
        qp.matchOnDescription = true;
        qp.matchOnDetail = true;

        let discovered: ProcessItem[] = [];
        const render = () => {
            const active = qp.activeItems[0]?.pid;
            const typedPid = /^\d+$/.test(qp.value.trim()) ? Number(qp.value.trim()) : undefined;
            const manual: ProcessItem[] =
                typedPid !== undefined && !discovered.some((i) => i.pid === typedPid)
                    ? [{ pid: typedPid, label: `$(debug-disconnect) Attach to PID ${typedPid}`, alwaysShow: true }]
                    : [];
            qp.items = [...manual, ...discovered];
            // Keep the highlighted process across refreshes.
            const keep = qp.items.find((i) => i.pid === active);
            if (keep) {
                qp.activeItems = [keep];
            }
        };

        let refreshing = false;
        let warned = false;
        const refresh = async () => {
            if (refreshing) {
                return;
            }
            refreshing = true;
            qp.busy = true;
            try {
                discovered = (await listProcesses(binary)).map(toItem);
                qp.title = `Select Python Process (${discovered.length} found)`;
            } catch (err) {
                qp.title = "Select Python Process (listing failed)";
                if (!warned) {
                    warned = true;
                    vscode.window.showWarningMessage(`FadeTop: could not list processes: ${err}`);
                }
            }
            qp.busy = false;
            refreshing = false;
            render();
        };
        // Same cadence as the TUI dashboard.
        const timer = setInterval(refresh, 2000);

        let done = false;
        qp.onDidChangeValue(render);
        qp.onDidTriggerButton((b) => b === refreshButton && refresh());
        qp.onDidAccept(() => {
            const item = qp.selectedItems[0];
            if (item) {
                done = true;
                resolve(item.pid);
                qp.hide();
            }
        });
        qp.onDidHide(() => {
            clearInterval(timer);
            if (!done) {
                resolve(undefined);
            }
            qp.dispose();
        });

        qp.title = "Select Python Process";
        qp.show();
        refresh();
    });
}
