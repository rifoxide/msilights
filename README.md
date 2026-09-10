# msilights

`msilights` is a Rust command-line tool for controlling supported MSI Mystic Light USB lighting controllers over HID feature reports.

The current implementation targets the common MSI USB device:

- Vendor ID: `0x0DB0`
- Product ID: `0x0076`
- HID feature report: `0x52`
- Normal report size: 185 bytes

> Hardware commands can change RGB state. Use `--dry-run` while testing command-line arguments and packet generation.

## Requirements

- Rust and Cargo
- Linux USB access to the MSI device
- The target MSI motherboard or controller connected over USB
- Permission to open and claim the USB HID interface

On Linux, USB permissions may require a udev rule or running the command with appropriate privileges. The program may detach an active kernel driver while it communicates with the device and attempts to reattach it when the transport is dropped.

## Build and test

```sh
cargo build
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

No ordinary test command writes to RGB hardware.

## Usage

Print the general help:

```sh
msilights --help
```

Running without a command also prints help:

```sh
msilights
```

List known zones:

```sh
msilights zones
```

List supported normal effects:

```sh
msilights effects
```

Use JSON output for metadata commands:

```sh
msilights --json zones
msilights --json effects
```

### Set one zone

Apply a temporary static orange color to one zone:

```sh
msilights set \
  --zone JRGB1 \
  --color FF3600 \
  --effect static \
  --brightness 90
```

The color may also include a leading `#`:

```sh
msilights set --zone JRGB1 --color '#FF3600'
```

Available writable zone names are currently:

- `JRGB1`
- `JRGB2`
- `PIPE1`
- `PIPE2`
- `JRAINBOW1`
- `JRAINBOW2`
- `ONBOARD`

### Set every writable zone

Apply one configuration to all currently writable zones:

```sh
msilights setall \
  --color FF3600 \
  --effect static \
  --brightness 90
```

`setall` updates the seven writable zones in one controller update. Metadata-only zones that are not currently mapped for writes are not targeted.

### Save behavior

By default, the setting is applied without requesting that the device save it to memory:

```sh
msilights setall --color FF3600 --effect static --brightness 90
```

Add `--save` to request persistent storage:

```sh
msilights setall --color FF3600 --effect static --brightness 90 --save
```

The `--save` flag controls the protocol save byte; it does not change the temporary hardware update sequence.

### Effects, speed, brightness, and secondary color

Effects can be specified by name or supported numeric value. Examples include:

```sh
msilights setall --color FF3600 --effect breathing --speed high --brightness 70
msilights setall --color FF3600 --secondary 0000FF --effect color-wave
```

Use `msilights effects` for the complete effect list.

Speed values:

- `low` or `0`
- `medium` or `1`
- `high` or `2`

Brightness values are percentage levels from `0` through `100` in steps of `10`, for example `70`, `90`, or `100`. `--secondary` defaults to the primary color when omitted.

## Dry-run mode

`--dry-run` validates the request and prints the generated 185-byte feature packet without opening the USB device or changing RGB state:

```sh
msilights setall \
  --color FF3600 \
  --effect static \
  --brightness 90 \
  --dry-run
```

It can be combined with JSON output:

```sh
msilights --json setall --color FF3600 --dry-run
```

Dry-run output includes the report ID, packet length, and encoded bytes.

## Safety and hardware notes

- Do not run non-dry-run commands unless the intended MSI device is connected.
- `set` and `setall` perform real USB writes.
- The normal update sequence reads the current feature report, sends the current report, then sends the desired report.
- Unsupported zones and invalid colors, effects, speeds, or brightness values are rejected before a report is sent.
- Hardware validation is intentionally opt-in; `cargo test` uses mock transports.

## Project status

Implemented:

- Checked 185-byte packet encoding and decoding
- HID feature-report transport with mock transport tests
- Device discovery for the common MSI USB product
- Normal zone color/effect/speed/brightness updates
- Save-byte handling
- Single-zone `set` and all-writable-zone `setall` commands
- Read-only zones/effects metadata commands
- Text and JSON metadata/packet output

The rewrite is still being developed. Direct per-LED mode, complete board-specific capability detection, firmware operations, and broader hardware validation remain future work.
