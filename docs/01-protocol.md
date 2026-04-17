# Protocolo USB del QUADRO

## Interfaces USB

El dispositivo (VID `0x0c70`, PID `0xf00d`) expone **dos interfaces USB**:

| Interface | Clase | Endpoints | Uso |
|---|---|---|---|
| 0 | Vendor Specific | EP `0x81 IN` bulk, EP `0x02 OUT` bulk | Escritura de virtual sensors (report 0x04) |
| 1 | HID | EP `0x83 IN` interrupt | Reports 0x01, 0x02, 0x03, 0x08 vía `/dev/hidrawN` |

Esto es crítico: **el report 0x04 NO se envía vía hidraw**. Cualquier intento de escribirlo con `HIDIOCSFEATURE` o `write()` sobre `/dev/hidraw0` falla con `EPIPE` porque hidraw sólo habla con Interface 1. Hay que usar USB bulk directo sobre `/dev/bus/usb/BBB/DDD` (USBDEVFS ioctls) o libusb.

## HID Reports (Interface 1)

Leídos del descriptor HID (`/sys/class/hidraw/hidraw0/device/report_descriptor`):

| Report ID | Tipo | Tamaño | Uso |
|-----------|------|--------|-----|
| `0x01` | INPUT | 220 bytes | Status report — sensores, RPM, voltaje... Se lee con `read()` del `/dev/hidrawX`. |
| `0x02` | OUTPUT | 11 bytes | Secondary/commit — enviado después de escribir ctrl report. |
| `0x03` | FEATURE | 961 bytes | **Control report** — config completa del device (fans, curvas, setup, etc). |
| `0x08` | FEATURE | 1013 bytes | **Labels** para sensores/fans/LEDs ("Fan 1", "Soft. Sensor 1"...). No es config. |

### Checksum CRC-16/USB

Tanto el control report (0x03) como el virtual sensors report (0x04) llevan checksum CRC-16/USB en los últimos 2 bytes. Cubre bytes `1..len-2` (excluye el report ID y los 2 bytes del propio checksum).

```rust
pub fn compute_checksum(buffer: &[u8]) -> u16 {
    let checksum_length = buffer.len() - 3;
    CRC_ALGO.checksum(&buffer[CHECKSUM_START..CHECKSUM_START + checksum_length])
}
```

## Report 0x04 — Virtual Sensors (USB bulk)

Tamaño: **67 bytes** (`0x43`). Se envía por EP `0x02 OUT` bulk en Interface 0.

```
Offset  Tamaño  Contenido
0x00    1       Report ID (0x04)
0x01    32      16 × u16 BE: valores de sensores virtuales (slots 0-15) en CENTI-GRADOS
                0x7FFF = "no disponible" (sensor desactivado)
0x21    16      16 × u8: tipos de sensor (0=Disabled, 3=Temperature, 5=Percentage, 7=Power)
0x31    16      16 × u8: bytes desconocidos. Aquasuite pone todos a 0x64.
0x41    2       CRC-16/USB checksum
```

- **Slots 0-7**: los 8 "software sensors" del firmware, usables en curvas de fans (ver [02-sensor-mapping.md](./02-sensor-mapping.md)).
- **Slots 8-15**: aparecen en el status report pero **NO son utilizables por las curvas**. Son los "Virtual Software Sensors" que Aquasuite calcula host-side.
- **Centi-grados**: `30.0°C` se escribe como `3000` (NO 30000 mC). Máximo representable: `65535 cd = 655.35°C`.
- **Los valores EXPIRAN**: si no se refrescan periódicamente (cada ~1-2s es seguro), el firmware los marca como inválidos y el fan cae al fallback. Empíricamente aguanta al menos 5s.

### Cómo escribirlo (ejemplo pyusb)

```python
import usb.core, usb.util
dev = usb.core.find(idVendor=0x0c70, idProduct=0xf00d)
usb.util.claim_interface(dev, 0)   # NO hacer detach_kernel_driver (no hace falta)
dev.write(0x02, report_bytes, timeout=2000)
usb.util.release_interface(dev, 0)
```

**⚠️ No llamar `detach_kernel_driver(0)`** — Interface 0 es Vendor Specific, no hay driver del kernel que la tenga cogida. Si haces detach, puedes dejar el dispositivo en estado inconsistente y requerir re-enumerar (`echo 0 > /sys/bus/usb/devices/.../authorized; echo 1 > ...`).

### Implementación Rust

Ya implementada en `src/device/linux.rs` via `USBDEVFS_BULK` ioctl directo sobre `/dev/bus/usb/BBB/DDD` (sin dependencia `libusb`/`rusb`). Resolución del path por VID/PID escaneando `/sys/bus/usb/devices`.

## Report 0x03 — Control Report

Tamaño: **961 bytes** (`0x3C1`). Checksum en `0x3BF-0x3C0`.

### Offsets principales (firmware 1033)

```
0x036  Fan 1 ctrl substructure
0x08B  Fan 2 ctrl substructure
0x0E0  Fan 3 ctrl substructure
0x135  Fan 4 ctrl substructure
0x18A  ... (8 estructuras misteriosas, ver §zona desconocida)
0x3BD  profile (1 byte)
0x3BF  CRC-16/USB checksum (2 bytes, big-endian)
```

### Estructura `Fan ctrl` (0x55 bytes = 85 bytes cada una)

```
+0x00  u8       Fan_ctrl_mode:
                  0 = PWM manual
                  1 = TEMP_TARGET (PID)
                  2 = CURVE
                  3-6 = FAN1-FAN4 (follow mode)
+0x01  u16 BE   pwm (centi-percent: 10000 = 100%) — usado en manual/fallback
+0x03  u16 BE   temp_sensor — índice del sensor fuente (ver 02-sensor-mapping.md)
+0x05  u16 BE   TEMP_TARGET.temp_target (centi-grados)
+0x07  u16 BE   TEMP_TARGET.P
+0x09  u16 BE   TEMP_TARGET.I
+0x0B  u16 BE   TEMP_TARGET.D1
+0x0D  u16 BE   TEMP_TARGET.D2
+0x0F  u16 BE   TEMP_TARGET.hysteresis
+0x11  2 bytes  padding (valor constante 0x0001 observado; propósito desconocido)
+0x13  u16 BE   CURVE.start_temp (centi-grados)
+0x15  32 bytes CURVE.temp[16]    (16 × u16 BE en CENTI-GRADOS)
+0x35  32 bytes CURVE.percent[16] (16 × u16 BE en centi-percent)
```

**Las temperaturas de la curva son centi-grados, NO millicelsius** (ver [03-findings-and-bugs.md](./03-findings-and-bugs.md)).

### Estructura `Fan setup` (9 bytes cada una, offsets 0x12, 0x1B, 0x24, 0x2D)

```
+0x00  u8       flags (bit 0 = hold_min_power, bit 1 = start_boost)
+0x01  u16 BE   min_percent (centi-percent)
+0x03  u16 BE   max_percent (centi-percent)
+0x05  u16 BE   fallback (centi-percent) — valor usado cuando el sensor no es válido
+0x07  u16 BE   graph_rpm (RPM usado en display scale)
```

### Zona desconocida 0x18A-0x3BC

560 bytes con **8 estructuras** de ~70 bytes. Tipos observados (primeros bytes):

| Struct | Offset | Bytes iniciales |
|---|---|---|
| 0 | 0x18A | `0f 03 00 00 ff ff` |
| 1 | 0x1D0 | `0f 0f 08 00 00 ff ff` |
| 2 | 0x217 | `0f 0b 00 00 ff ff` |
| 3 | 0x25D | `0f 04 00 06 ff ff` |
| 4 | 0x2A3 | `0f 04 00 06 ff ff` |
| 5 | 0x2E9 | `0f 04 00 06 ff ff` |
| 6 | 0x32F | `0f 00 00 06 ff ff` |
| 7 | 0x375 | `0f 00 00 06 ff ff` |

**Hipótesis (sin confirmar)**: configuraciones de los 8 software sensors (scale factor, offset, data source type). Aquasuite las escribe cuando el usuario configura cada software sensor.

**No hace falta manipular estos bytes** para nuestro caso de uso: basta con escribir el valor via report 0x04 y la curva lo lee.

## Report 0x01 — Status Report

Tamaño: 220 bytes (`0xDC`). Se lee con `read()` sobre el fd de `/dev/hidrawX`.

### Offsets principales

```
0x03  serial part 1 (u16 BE)
0x05  serial part 2 (u16 BE)
0x0D  firmware version (u16 BE)
0x18  power cycles (u32 BE)
0x34  temp sensor 1 (u16 BE centi-grados; 0x7FFF = desconectado)
0x36  temp sensor 2
0x38  temp sensor 3
0x3A  temp sensor 4
0x3C  virt_sensor_val[16] (16 × u16 BE centi-grados)
0x5C  virt_sensor_type[16] (16 × u8: 0/3/5/7)
0x6E  flow sensor
0x70  Fan 1 substructure (RPM, voltage, current, power, speed, torque — 14 bytes)
0x7D  Fan 2
0x8A  Fan 3
0x97  Fan 4
```

### Estructura Fan (status)

```
+0x00  u16 BE  speed (0-100%, ese ES el "pwm" reportado en nuestro JSON)
+0x02  u16 BE  voltage (centi-V)
+0x04  u16 BE  current (centi-A)
+0x06  u16 BE  power (centi-W)
+0x08  u16 BE  RPM
+0x0A  u16 BE  torque
```

## Unidades en el wire (resumen)

| Campo | Unidad | Ejemplo |
|---|---|---|
| Curve temp points (report 0x03) | centi-grados | `20.0°C = 2000` |
| start_temp (report 0x03) | centi-grados | `28.0°C = 2800` |
| Physical sensor reading (report 0x01) | centi-grados | `22.91°C = 2291` |
| Virtual sensor value (report 0x01 y 0x04) | centi-grados | `30.0°C = 3000` |
| Flow correction factor | centi-factor | `1.00 = 100` |
| PWM / percentage | centi-percent | `50% = 5000` |
| Voltage | centi-V | `12.15V = 1215` |
| RPM | raw u16 | `2000 RPM = 2000` |

**Toda la telemetría de temperatura usa centi-grados. No hay millicelsius en ninguna parte.**
