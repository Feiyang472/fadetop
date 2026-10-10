// Build fadetop with cargo and package a VSIX for the current platform.
import { execFileSync } from "child_process";
import { copyFileSync, mkdirSync } from "fs";
import { fileURLToPath } from "url";

const root = fileURLToPath(new URL("../..", import.meta.url));
const ext = fileURLToPath(new URL("..", import.meta.url));
const exe = process.platform === "win32" ? "fadetop.exe" : "fadetop";
const target = `${process.platform}-${process.arch === "arm" ? "armhf" : process.arch}`;

const run = (cmd, args, cwd) => execFileSync(cmd, args, { cwd, stdio: "inherit", shell: process.platform === "win32" });

run("cargo", ["build", "--release", "--bin", "fadetop"], root);
mkdirSync(`${ext}/bin`, { recursive: true });
copyFileSync(`${root}/target/release/${exe}`, `${ext}/bin/${exe}`);
run("npx", ["vsce", "package", "--target", target, "-o", `fadetop-${target}.vsix`], ext);
