import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { discoverPythonProcesses, displayName, PythonProcess } from "./processes";

export function activate(context: vscode.ExtensionContext) {
    context.subscriptions.push(
        vscode.commands.registerCommand("fadetop.openDashboard", () => launch(context, [], "FadeTop")),
        vscode.commands.registerCommand("fadetop.attach", async (pid?: number) => {
            pid ??= await pickProcess();
            if (pid !== undefined) {
                await launch(context, [String(pid)], `FadeTop: ${pid}`);
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

const HOLD_ON_FAILURE =
    '"$@"; code=$?; if [ "$code" -ne 0 ]; then ' +
    'printf "\\n[fadetop exited with code %s, press Enter to close]" "$code"; read -r _; fi';

async function launch(context: vscode.ExtensionContext, args: string[], name: string) {
    const binary = resolveBinary(context);
    if (!binary) {
        return;
    }
    const config = vscode.workspace.getConfiguration("fadetop");
    let useSudo = config.get<boolean>("useSudo", false) && process.platform !== "win32";
    const ptrace = await checkPtrace(useSudo);
    if (ptrace === "cancel") {
        return;
    }
    useSudo ||= ptrace === "sudo";

    const env = configEnv();
    let shellPath = binary;
    let shellArgs = args;
    if (process.platform !== "win32") {
        let command = [binary, ...args];
        if (useSudo) {
            // sudo resets the environment, so pass settings through `env`.
            command = ["sudo", "env", ...Object.entries(env).map(([k, v]) => `${k}=${v}`), ...command];
        }
        // Keep the terminal open on failure so the error can be read.
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

interface ProcessItem extends vscode.QuickPickItem {
    pid: number;
}

function toItem(p: PythonProcess): ProcessItem {
    const name = displayName(p);
    return {
        pid: p.pid,
        label: `$(pulse) ${name}`,
        description: `PID ${p.pid}`,
        detail: name === p.cmdline ? undefined : p.cmdline,
    };
}

/** Quick-pick mirror of the TUI dashboard's process selection list. */
function pickProcess(): Promise<number | undefined> {
    return new Promise((resolve) => {
        const qp = vscode.window.createQuickPick<ProcessItem>();
        const refreshButton: vscode.QuickInputButton = {
            iconPath: new vscode.ThemeIcon("refresh"),
            tooltip: "Refresh",
        };
        qp.buttons = [refreshButton];
        qp.placeholder = "Filter by command line, or type a PID";
        qp.matchOnDescription = true;
        qp.matchOnDetail = true;

        let discovered: ProcessItem[] = [];
        const render = () => {
            const typedPid = /^\d+$/.test(qp.value.trim()) ? Number(qp.value.trim()) : undefined;
            const manual: ProcessItem[] =
                typedPid !== undefined && !discovered.some((i) => i.pid === typedPid)
                    ? [{ pid: typedPid, label: `$(debug-disconnect) Attach to PID ${typedPid}`, alwaysShow: true }]
                    : [];
            qp.items = [...manual, ...discovered];
        };
        const refresh = async () => {
            qp.busy = true;
            try {
                discovered = (await discoverPythonProcesses()).map(toItem);
            } catch (err) {
                discovered = [];
                vscode.window.showWarningMessage(`FadeTop: could not list processes: ${err}`);
            }
            qp.title = `Select Python Process (${discovered.length} found)`;
            qp.busy = false;
            render();
        };

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
