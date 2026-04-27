# quadro-ctl

CLI tool for bulk read/write operations on Aqua Computer fan controllers: [QUADRO](https://www.aquacomputer.de/quadro.html) (4 fans) and [OCTO](https://www.aquacomputer.de/octo.html) (8 fans).

Controls fans (manual PWM or temperature curve) and feeds the firmware's **software sensors** with values read from the host OS (HDD temps, NVMe, CPU, etc.) so a curve can react to anything Linux can measure.

## Why

The sysfs/hwmon interface performs individual USB round-trips for each operation (~9 seconds each). Configuring all fans via sysfs takes ~20 minutes (QUADRO) or ~40 minutes (OCTO). `quadro-ctl` talks to the device directly: one HID feature-report read + one feature-report write + one commit report (~18 seconds for the whole configuration).

It also supports things the kernel driver doesn't:

- Selecting **software sensors (5-12)** or the **flow sensor (4)** as the curve input — the kernel driver only allows the 4 hardware sensors (0-3).
- Writing **software sensor values** (report 0x04 via USB bulk on Interface 0 EP 0x02 OUT). The kernel driver doesn't expose this at all; `quadro-ctl` sends it directly via `USBDEVFS_BULK`.

## Commands

| Command | What it does |
|---------|--------------|
| `fans get` | Read the control report (fan modes, curves, sensor selection). Prints JSON. |
| `fans set --config-file F` | Read control report, merge in the fans from `F`, write back, commit. |
| `sensors set --config-file F` | Write software sensor values over USB bulk (report 0x04). |
| `status` | Full device snapshot: sensor values (hardware + virtual), fan RPM/PWM/voltage/current, flow. Prints JSON. |

Every command accepts `--device /dev/hidrawN` to pin a specific device. Without it, `quadro-ctl` scans `/dev/hidraw*` for VID `0x0c70` and detects QUADRO (PID `0xf00d`) or OCTO (PID `0xf011`).

Logs go to **stderr**; data (JSON) goes to **stdout** — pipe safely.

## Usage

### `status` — read sensors, RPMs, PWMs

```sh
sudo quadro-ctl status
```

```json
{
  "device": { "serial": "32533-07983", "firmware": 1033, "power_cycles": 27 },
  "temperatures": {
    "sensor1": 22.42, "sensor2": null, "sensor3": null, "sensor4": null,
    "virtual1": 30.0, "virtual2": null, ...
  },
  "fans": {
    "fan1": { "rpm": 10115, "pwm": 10000, "voltage": 12.15, "current": 0.31, "power": 0.37 },
    ...
  },
  "flow": 0.0
}
```

`sensor1..sensor4` are the hardware temperature inputs (4 on both QUADRO and OCTO). `virtual1..virtual16` are the firmware's 16 software-sensor slots (see below). OCTO shows 8 fans; QUADRO shows 4.

### `fans get` — inspect the current fan configuration

```sh
sudo quadro-ctl fans get
```

```json
{
  "fans": {
    "fan1": {
      "mode": "curve",
      "sensor": 5,
      "points": [
        { "temp": 20.0, "percentage": 20 },
        { "temp": 22.0, "percentage": 25 },
        ...16 points total...
        { "temp": 50.0, "percentage": 95 }
      ]
    },
    "fan2": { "mode": "manual", "percentage": 40 }
  }
}
```

### `fans set` — write fan modes / curves

The config is **partial**: only the fans you include are changed. Unconfigured fans (and the rest of the control report) are preserved.

Manual mode:

```json
{ "fans": { "fan2": { "mode": "manual", "percentage": 60 } } }
```

Curve mode (fan1 follows software sensor 1):

```json
{
  "fans": {
    "fan1": {
      "mode": "curve",
      "sensor": 5,
      "points": [
        { "temp": 20.0, "percentage": 20 },
        { "temp": 22.0, "percentage": 25 },
        { "temp": 24.0, "percentage": 30 },
        { "temp": 26.0, "percentage": 35 },
        { "temp": 28.0, "percentage": 40 },
        { "temp": 30.0, "percentage": 45 },
        { "temp": 32.0, "percentage": 50 },
        { "temp": 34.0, "percentage": 55 },
        { "temp": 36.0, "percentage": 60 },
        { "temp": 38.0, "percentage": 65 },
        { "temp": 40.0, "percentage": 70 },
        { "temp": 42.0, "percentage": 75 },
        { "temp": 44.0, "percentage": 80 },
        { "temp": 46.0, "percentage": 85 },
        { "temp": 48.0, "percentage": 90 },
        { "temp": 50.0, "percentage": 95 }
      ]
    }
  }
}
```

```sh
sudo quadro-ctl fans set --config-file fan1.json
```

Constraints:

- Curves require **exactly 16 points**.
- Temperatures must be **strictly increasing** (monotonic).
- `temp` is in **°C** (with 0.01°C resolution; range 0.0–655.35).
- `percentage` is 0–100.
- `sensor` is an integer — see the sensor mapping below.

### `sensors set` — feed software sensors

```sh
echo '{"virtual1": 30.0, "virtual3": 45.5}' > vs.json
sudo quadro-ctl sensors set --config-file vs.json
```

This writes report `0x04` over USB bulk (Interface 0, EP 0x02 OUT). Keys are `virtual1` … `virtual16`, values are °C.

**Important behaviors:**

- **Every call is an absolute overwrite.** Sensors not listed in the file are marked *disabled* on the device. To keep virtual1 and virtual2 both alive, include both in every call.
- **Values expire.** The firmware invalidates sensor values if they are not refreshed; in testing they persist for at least ~5 s, so refreshing every 1–2 s is safe. A fan in curve mode whose sensor has expired will fall back to its minimum setting.
- **Only `virtual1`..`virtual8` can drive fan curves.** Slots 9–16 exist and show up in `status`, but the firmware doesn't let the curves read them (those slots are "calculated virtual sensors" in Aquasuite's terminology).

Typical wiring for a NAS: a systemd timer runs every 2 s, reads HDD/NVMe/CPU temps from `hwmon` or `smartctl`, builds the JSON, and calls `sensors set`. Fans configured in `curve` mode with `sensor: 5..12` follow those values.

## Sensor index mapping

The `sensor` field in a curve config selects the input for the fan:

| Value  | Meaning |
|--------|---------|
| 0–3    | Hardware temperature sensors 1–4 (the 2-pin inputs on the device) |
| 4      | Flow sensor |
| 5–12   | Software sensors 1–8 (correspond to `virtual1`..`virtual8` in `status`) |
| 13–19  | Invalid / reserved — fan will fall back |
| 65535  | "No sensor" — fan will fall back |

## Units on the wire (for reference)

The device uses centi-units almost everywhere:

- Curve temperatures, status temperatures, virtual-sensor writes: **centi-°C** (×100). `30.0 °C = 3000`.
- PWM / percentages: **centi-%** (×100). `50% = 5000`.
- Flow rate in `status`: deci-L/h (÷10).

`quadro-ctl` does these conversions for you; configs are written in human units (°C, %).

## Running without root

Both HID feature-report ioctls and USB bulk writes need privileged access by default. Either run under `sudo`, or install udev rules that grant access to the device (`/dev/hidraw*` and `/dev/bus/usb/<BUS>/<DEV>`) for a specific group.

## Building

```sh
cargo build --release
```

Linux-only (uses `hidraw` and `usbdevfs` ioctls). Tested on x86_64 and aarch64.

## Reference

- Kernel driver: [aquacomputer_d5next-hwmon](https://github.com/aleksamagicka/aquacomputer_d5next-hwmon) (hardware-sensor curves only; doesn't cover software sensors).
- Python reference project: [leoratte/aquacomputer-quadro-control](https://github.com/leoratte/aquacomputer-quadro-control) — confirms centi-°C units and layout.
- Protocol details and reverse-engineering notes: see `RESEARCH.md`.
