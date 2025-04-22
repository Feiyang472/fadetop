# FadeTop

FadeTop is a real-time in-terminal visualiser for Python stack samples.

![](https://github.com/Feiyang472/fadetop/actions/workflows/build.yml/badge.svg)

Watch as call stacks are entered and exited, threads get spawn and destroyed, iterations proceed, or loss functions optimised.
![Demo](https://raw.githubusercontent.com/Feiyang472/fadetop/refs/heads/main/.github/local.gif)

FadeTop relies on **py-spy** for generating stack traces and **ratatui** for its front-end interface.

## Usage
To use FadeTop, run the following command:

```sh
fadetop $PID_OF_YOUR_RUNNING_PYTHON_PROCESS
```

Replace `$PID_OF_YOUR_RUNNING_PYTHON_PROCESS` with the process ID of the Python program you want to analyze.

## Installation
Fadetop is published to pypi as a binary package under name `pyfadetop` (the binary will still be `fadetop`).
Binaries are built for linux, macos, and windows.
```
pip install pyfadetop
```

Alternatively fadetop can be built from source using `cargo build`.

## Configuration
Fadetop can be configured using both a toml file (named `fadetop_config.toml` or `$FADETOP_CONFIG` if set) and environment variables, where the latter overrides the former.

You can check your configuration by running `fadetop --help`

## Configuration Examples

### Example using TOML Config File
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

### Example using Environment Variables
```bash
export FADETOP_SAMPLING_RATE=120
```
