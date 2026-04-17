# Testing y deploy al NAS

## Flujo de deploy

Ver también la memoria persistente en `~/.claude/projects/-Users-danielramos-Documents-repos-infra-quadro-ctl/memory/deploy_workflow.md`.

```bash
# 1. Sync local → NAS (excluye el target/ para no mandar GBs)
rsync -avz --exclude target /Users/danielramos/Documents/repos/infra/quadro-ctl/ nas:/home/dani/quadro-ctl/

# 2. Build en el NAS (es ARM/Linux nativo, cross-compilación no requerida)
ssh nas 'cd /home/dani/quadro-ctl && cargo build --release'

# 3. Ejecutar (requiere sudo para acceder a /dev/hidraw* y /dev/bus/usb/*)
ssh nas 'sudo /home/dani/quadro-ctl/target/release/quadro-ctl <comando>'
```

## Tests automatizados (locales)

```bash
cargo test
```

Debería pasar 125+ tests entre unit e integración. Ninguno requiere hardware físico — usan el `MockDeviceFactory`.

## Scripts manuales disponibles en el NAS

Están en `/tmp/` (no persistentes entre reboots del NAS). Se recrean al ejecutar ciertos comandos desde local. Si ya no están, ver las definiciones más abajo y recrearlas con `ssh nas 'cat > /tmp/X.py << EOF ... EOF'`.

### Escritura de virtual sensors via pyusb

**`/tmp/test_vs.py <celsius>`** — escribe una temperatura al slot 0 (= `virtual1` = software sensor 1 en curvas con `sensor: 5`).

```python
import sys, crcmod, usb.core, usb.util

VENDOR, PRODUCT, REPORT_SIZE, EP_OUT = 0x0c70, 0xf00d, 0x43, 0x02

def build_report(sensor_idx, centi_degrees):
    buf = bytearray(REPORT_SIZE)
    buf[0] = 0x04
    for i in range(16):
        off = 0x01 + i * 2
        buf[off] = 0x7F; buf[off + 1] = 0xFF
        buf[0x21 + i] = 0x00
    off = 0x01 + sensor_idx * 2
    buf[off] = (centi_degrees >> 8) & 0xFF
    buf[off + 1] = centi_degrees & 0xFF
    buf[0x21 + sensor_idx] = 0x03
    for i in range(16):
        buf[0x31 + i] = 0x64
    crc = crcmod.predefined.mkCrcFun("crc-16-usb")(bytes(buf[1:0x41]))
    buf[0x41] = (crc >> 8) & 0xFF
    buf[0x42] = crc & 0xFF
    return bytes(buf)

celsius = float(sys.argv[1])
centi = int(celsius * 100)
dev = usb.core.find(idVendor=VENDOR, idProduct=PRODUCT)
usb.util.claim_interface(dev, 0)
try:
    report = build_report(0, centi)
    dev.write(EP_OUT, report, timeout=2000)
    print(f"virtual1 = {celsius}°C written (67 bytes)")
finally:
    usb.util.release_interface(dev, 0)
```

Ejecución:

```bash
sudo nix-shell -p python3Packages.pyusb python3Packages.crcmod --run "python3 /tmp/test_vs.py 30.0"
```

### Inspeccionar bytes raw del control report

**`/tmp/inspect_fan.py`** — dumpea mode, pwm, temp_sensor, start_temp, primeros 4 puntos de curva para fan1 y fan4.

```python
import struct, fcntl
fd = open("/dev/hidraw0", "r+b", buffering=0)
buf = bytearray(961)
buf[0] = 0x03
HIDIOCGFEATURE = (3 << 30) | (961 << 16) | (ord("H") << 8) | 0x07
fcntl.ioctl(fd, HIDIOCGFEATURE, buf)

def u16(off):
    return struct.unpack_from(">H", buf, off)[0]

for label, base in [("fan1", 0x36), ("fan4", 0x135)]:
    print(f"{label} @ 0x{base:x}:")
    print(f"  mode: {buf[base]}")
    print(f"  pwm: {u16(base+1)}")
    print(f"  temp_sensor: {u16(base+3)}")
    print(f"  start_temp: {u16(base+0x13)}")
    temps = [u16(base+0x15+i*2) for i in range(4)]
    pcts = [u16(base+0x35+i*2) for i in range(4)]
    print(f"  first 4 temps (cd): {temps}")
    print(f"  first 4 pcts (cpct): {pcts}")
```

Ejecución: `ssh nas 'sudo python3 /tmp/inspect_fan.py'`

### Re-enumerar el USB si queda en estado raro

Si el dispositivo deja de responder a `HIDIOCGFEATURE` (error EPIPE):

```bash
# Encontrar el path sysfs
DEV=$(for d in /sys/bus/usb/devices/*/idProduct; do
    if [ "$(cat $d)" = "f00d" ]; then
        dirname $d | xargs basename
    fi
done)
echo "0" | sudo tee /sys/bus/usb/devices/$DEV/authorized
sleep 3
echo "1" | sudo tee /sys/bus/usb/devices/$DEV/authorized
sleep 3
# Reintentar config
echo "1" | sudo tee /sys/bus/usb/devices/$DEV/bConfigurationValue
# Verificar que /dev/hidraw0 volvió
ls /dev/hidraw*
```

### Lectura del status report (debugging bajo nivel)

**`/tmp/read_status.py`** — lee y dumpea los 220 bytes del status report.

```python
import struct, os
fd = os.open("/dev/hidraw0", os.O_RDONLY)
buf = os.read(fd, 220)
def u16(off): return struct.unpack_from(">H", buf, off)[0]
print(f"fw: {u16(0x0D)}  power_cycles: {struct.unpack_from('>I', buf, 0x18)[0]}")
print(f"temp sensors: {[u16(0x34+i*2) for i in range(4)]}")
print(f"virt vals: {[u16(0x3C+i*2) for i in range(16)]}")
print(f"virt types: {[buf[0x5C+i] for i in range(16)]}")
```

### Descubrir rápido el mapeo ts↔slot (script de probe)

Si en el futuro aparece FW nuevo y el mapeo cambia, este es el script que lo descubrió:

```bash
for TS in 5 6 7 8 9 10 11 12; do
for SLOT in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
    # escribir curva con sensor=TS
    # escribir virtual SLOT con 15°C, leer fan (LOW)
    # escribir virtual SLOT con 50°C, leer fan (HIGH)
    # si delta > 500, es match
done
done
```

El pattern esperado: `TS=5 SLOT=0`, `TS=6 SLOT=1`, ..., `TS=12 SLOT=7` — con la misma magnitud de delta.

## Precauciones

- **fan4 siempre en manual ≥ 50% durante pruebas**. Está cerca de los HDDs. Una config rota puede apagarlo silenciosamente y los discos suben de temperatura rápido.
- **Verificar estados intermedios** (`quadro-ctl fans get`, `quadro-ctl status`) antes de asumir que algo funciona. El firmware a veces acepta valores raros sin error.
- **Los poke scripts directos bypassan la validación de Rust**. Cuidado con valores de `temp_sensor > 12` (deja el fan en fallback) o curvas con `percentage > 10000` (posible corrupción interna).

## Estado del hardware (último conocido)

Al cierre de la sesión de investigación:

- Los 4 fans estaban en modo manual al 100%.
- Firmware version `1033`, power_cycles `21-27`.
- Serial `32533-07983`.
