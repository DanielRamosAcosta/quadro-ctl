# Siguiente fase de trabajo

> Antes de empezar, lee los otros docs de esta carpeta — especialmente [02-sensor-mapping.md](./02-sensor-mapping.md) y [03-findings-and-bugs.md](./03-findings-and-bugs.md).

## Estado actual del código

Ya refactorizado y compilando:

- ✅ `Temperature` usa **centi-grados** internamente (`×100`). Range 0..=655.35°C.
- ✅ `Millicelsius` eliminado completamente.
- ✅ `CurveData.temps` pasó a `[Temperature; 16]`.
- ✅ `buffer.rs`, `raw_report.rs`, `curve_data.rs`, `tests/integration.rs` actualizados.
- ✅ `SensorIndex` soporta 0-19 (suficiente para los válidos 0-12).
- ✅ `RawVirtualSensorsReport` construye el buffer 67-byte del report 0x04 con CRC correcto.
- ✅ `VirtualSensorsConfig` con deserialización `{"virtual1": 45.0, ...}`.
- ✅ `write_virtual_sensors` en `src/device/linux.rs` usa **USBDEVFS_BULK ioctl** sobre `/dev/bus/usb/BBB/DDD` (Interface 0, EP `0x02 OUT`). Sin `libusb`/`rusb`.
- ✅ CLI reestructurada:
  - `quadro-ctl fans get` (antes `read`)
  - `quadro-ctl fans set --config-file ...` (antes `apply`)
  - `quadro-ctl sensors set --config-file ...` (antes `set-virtual-sensors`)
  - `quadro-ctl status`

## Lo que falta

### Paso 1 — Validación end-to-end en el NAS

Sync + build + probar el binario actualizado:

```bash
rsync -avz --exclude target /Users/danielramos/Documents/repos/infra/quadro-ctl/ nas:/home/dani/quadro-ctl/
ssh nas 'cd /home/dani/quadro-ctl && cargo build --release'
```

**Test plan**:

1. `quadro-ctl fans get` — debe mostrar las curvas (si hay alguna) con temps en grados decimales.
2. Aplicar fan1 en modo curva con `sensor: 5` y una curva conocida, ej:

   ```json
   {
     "fans": {
       "fan1": {
         "mode": "curve",
         "sensor": 5,
         "points": [
           {"temp": 20.0, "percentage": 20},
           {"temp": 25.0, "percentage": 40},
           {"temp": 30.0, "percentage": 60},
           {"temp": 35.0, "percentage": 80},
           {"temp": 40.0, "percentage": 100},
           ...16 puntos
         ]
       }
     }
   }
   ```
3. `quadro-ctl sensors set --config-file <json>` con `{"virtual1": 30.0}`. Verificar en `quadro-ctl status` que `virtual1: 30.0` aparece.
4. Observar fan1 en `status` — debe estar al ~60% PWM.
5. Escribir `{"virtual1": 40.0}` — fan1 debe subir al ~100%.
6. Esperar 10 segundos sin reescribir — fan debe caer al fallback (probablemente 100%).

⚠️ **Siempre** dejar fan4 en manual al 100% durante estas pruebas: está cerca de los HDDs y si queda a 0% se cocinan.

### Paso 2 — Implementar `quadro-ctl watch`

Los software sensors expiran. Hace falta un daemon que:

1. Lea temperaturas de Linux (nvme/HDDs) periódicamente.
2. Mapee cada temperatura a un software sensor del QUADRO.
3. Escriba el reporte 0x04 con todos los valores.
4. Duerma ~1-2 segundos y repita.

**Propuesta de diseño** (sujeto a discusión con el usuario):

```rust
// Nuevo subcomando
quadro-ctl watch --config-file watch.yaml
```

Formato `watch.yaml`:

```yaml
interval: 2s
sensors:
  virtual1:
    source: hwmon
    path: /sys/class/hwmon/hwmon3/temp1_input  # nvme0
    scale: 0.001  # millidegrees → grados
  virtual2:
    source: hwmon
    path: /sys/class/hwmon/hwmon4/temp1_input  # nvme1
    scale: 0.001
  virtual3:
    source: smartctl
    device: /dev/sda
    attribute: 194  # Temperature_Celsius
  virtual4:
    source: shell
    command: "smartctl -A /dev/sda /dev/sdb /dev/sdc /dev/sdd -j | jq 'max of HDD temps'"
```

Alternativa más simple: un script bash/python externo que genere un JSON y lo pipee a `quadro-ctl sensors set --config-file /dev/stdin`, y un systemd timer que lo lance.

Mi recomendación: empezar con el script externo + timer (más simple de iterar), y si se vuelve feo migrar a `watch` interno.

### Paso 3 — Módulo NixOS `services.quadro-fans`

Repo consumer: `/Users/danielramos/Documents/repos/infra/nas`.

Después del paso 2, el módulo debe:

1. **Al boot**: aplicar `quadro-ctl fans set` con una config base. **Importante**: como los software sensors tardan un momento en estar vivos (el daemon aún no ha arrancado), la config de boot debería usar **sensor físico 0** (sensor1) como fallback seguro, NO un software sensor. Algo tipo "mínimo aceptable siempre".

2. **Servicio systemd**: lanzar `quadro-ctl watch` (o el script externo) con restart automático.

3. **Segundo apply** tras el primer tick del watch: volver a aplicar la config "correcta" con `sensor: 5..12` apuntando a software sensors. Así si el watch muere, el último apply de boot (sensor físico) sigue siendo seguro.

Alternativa más robusta: configurar las curvas con `fallback: 100` en `fan_setup`, así aunque los software sensors mueran, el fallback garantiza flujo de aire. El apply de boot ya no importa tanto.

### Paso 4 — Pulido y testing

- Tests de integración para el path de virtual sensors.
- Tests mock para `UsbBulkDevice` (actualmente sólo tiene mock via `MockDeviceFactory`).
- Documentar el formato del config YAML del watch si se hace interno.
- Actualizar `README.md` principal del repo con las nuevas capacidades.

## Detalles técnicos finos

### Fallback seguro en los `fan_setup`

Observado en el dispositivo actual:

```
fan1 setup: flags=2 min=500 max=10000 fallback=10000 graph_rpm=2000
```

`fallback = 10000` (100%) — esto es lo que hace el fan cuando el sensor asignado no es válido. Bueno para seguridad: si el software sensor expira, el fan sube al 100% en vez de apagarse.

Si queremos ser más agresivos con el silencio, bajar `fallback` a ~5000 (50%) mantendría los discos razonablemente frescos sin hacer ruido si el watch muere momentáneamente.

### Chequeo rápido de que virtual sensors funcionan tras el refactor

Si tras el paso 1 algo no funciona, ejecutar en el NAS:

```bash
sudo nix-shell -p python3Packages.pyusb python3Packages.crcmod --run "python3 /tmp/test_vs.py 30.0"
sudo /home/dani/quadro-ctl/target/release/quadro-ctl status | grep virtual1
```

Si el status muestra `virtual1: 30.0`, la parte de escritura via pyusb funciona (el problema estaría en nuestra implementación Rust). Si no lo muestra, el dispositivo está en estado raro y hay que re-enumerar:

```bash
DEV=$(for d in /sys/bus/usb/devices/*/idProduct; do if [ "$(cat $d)" = "f00d" ]; then dirname $d | xargs basename; fi; done)
echo 0 | sudo tee /sys/bus/usb/devices/$DEV/authorized
sleep 2
echo 1 | sudo tee /sys/bus/usb/devices/$DEV/authorized
```

Ver más scripts de test en [05-testing.md](./05-testing.md).
