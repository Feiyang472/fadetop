# FadeTop

FadeTop is a real-time in-terminal visualiser for Python stack samples.

![](https://github.com/Feiyang472/fadetop/actions/workflows/build.yml/badge.svg)

Watch as call stacks are entered and exited, threads get spawn and destroyed, iterations proceed, or loss functions optimised.
![Demo](.github/local.gif)

FadeTop relies on **py-spy** for generating stack traces and **ratatui** for its front-end interface.

## Usage
To use FadeTop, run the following command:

```sh
fadetop $PID_OF_YOUR_RUNNING_PYTHON_PROCESS
```

Replace `$PID_OF_YOUR_RUNNING_PYTHON_PROCESS` with the process ID of the Python program you want to analyze.

## Installation [Work in Progress]
I am working on packaging a binary with maturin for distribution via PyPI.
Currently the only way to install is to clone this repository and `cargo build`.

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
```

### Example using Environment Variables
```bash
export FADETOP_SAMPLING_RATE=120
```
