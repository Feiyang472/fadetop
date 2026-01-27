# FadeTop

FadeTop is a real-time in-terminal visualiser for Python stack samples.

![](https://github.com/Feiyang472/fadetop/actions/workflows/build.yml/badge.svg)

Watch as call stacks are entered and exited, threads spawn and terminate, iterations proceed, and loss functions get optimised.
![Demo](https://raw.githubusercontent.com/Feiyang472/fadetop/refs/heads/main/.github/local.gif)

FadeTop relies on **py-spy** for generating stack traces and **ratatui** for its front-end interface.

## Usage

```sh
# Attach to a specific Python process by PID
fadetop $PID_OF_YOUR_RUNNING_PYTHON_PROCESS

# Or run without arguments to open a dashboard that discovers running Python processes
fadetop

# Run the demo with mock data (no real process needed)
fadetop demo
```

## Installation

FadeTop is published to PyPI as `pyfadetop`. Pre-built binaries are available for Linux, macOS, and Windows.

Run directly with uv:
```sh
uvx pyfadetop
```

Or install it:
```sh
uv pip install pyfadetop
# or
pip install pyfadetop
```

Alternatively, build from source with `cargo build`.

### Permissions

FadeTop requires the same permissions as py-spy for [ptrace](https://github.com/benfred/py-spy?tab=readme-ov-file#when-do-you-need-to-run-as-sudo). On Linux, either:
- Set `/proc/sys/kernel/yama/ptrace_scope` to `0`, or
- Run as `sudo`

In Docker/Kubernetes environments, add `--cap-add=SYS_PTRACE` to your container.

## Configuration

FadeTop can be configured via a TOML file (`fadetop_config.toml`, or set `$FADETOP_CONFIG`) and environment variables. Environment variables take precedence.

Check your current configuration with `fadetop show-config`.

### Examples

TOML config file:
```toml
# Sampling rate in Hz (samples per second)
sampling_rate = 120
# Time window width for visualization
window_width = "100s"

# Rules dictate how long events are remembered after they have finished as a function of how long they took to run.
# The config below means an event is remembered for the shorter interval between (100 seconds + three times its duration) and (70s + 1.0 times its duration)

[[rules]]
type = "rectlinear"
at_least = "100s"
ratio = 3.0
[[rules]]
type = "rectlinear"
at_least = "70s"
ratio = 1.0
```

Environment variables:
```bash
export FADETOP_SAMPLING_RATE=120
```
