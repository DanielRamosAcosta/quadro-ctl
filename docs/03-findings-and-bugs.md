# Hallazgos y bugs descubiertos

## Bug crítico: centi-grados vs millicelsius en las curvas

**Todas las temperaturas del protocolo QUADRO son centi-grados (×100), NO millicelsius (×1000).**

### Síntomas antes del fix

- Fans en modo curva no respondían predeciblemente a la temperatura del sensor.
- Al aplicar curva `[15, 17, 19, 21, 23...]°C` y observar sensor1 a 20.91°C, el fan iba al 28.75% (cerca del primer punto, no interpolación correcta).
- Con `sensor=5-12` (software sensors), el fan siempre quedaba estancado en el mismo valor (~14.5%) sin importar la temperatura escrita al virtual sensor.

### Causa

El value object `Temperature` almacenaba millicelsius internamente y exportaba `to_millicelsius()` tal cual al wire. Al escribir `20.0°C` como `20000`, el firmware lo interpretaba como `200.00°C` (ya que espera centi-grados). Todos los puntos de curva quedaban absurdamente altos, y el sensor (`2091 = 20.91°C` en centi-grados) siempre caía por debajo del primer punto.

### Fix aplicado (ya está en `src/protocol/temperature.rs`)

- `Temperature` ahora almacena **centi-grados** internamente.
- `from_celsius(f64)` multiplica por `100`.
- `to_centi_degrees() -> u16` devuelve el valor directo del wire.
- Rango válido: `0.0 ..= 655.35°C` (max u16 = 65535 centi-grados).
- Serde ↔ JSON sigue usando grados decimales (`20.5`).
- El tipo `Millicelsius` fue **eliminado** por completo.

### Evidencia definitiva

Tras el fix, con curva `[10=20%, 20=45%, 30=65%, 40=100%]` y sensor1 a 22.44°C:
- Interpolación esperada: entre 22°C=50% y 24°C=55% → ~51%
- Observado: fan1 a **53.54%** ✓ (la pequeña diferencia se debe al redondeo de interpolación del firmware)

### Referencia externa que lo confirmó

El proyecto [leoratte/aquacomputer-quadro-control](https://github.com/leoratte/aquacomputer-quadro-control) (Python, GUI para Linux, probado en FW 1028). Su `src/quadrocontrol/converter.py` usa `factor=100` para **todas** las temperaturas:

```python
config.fans[i].curve_mode_vars.temp[x] = self.convert(2, 100)  # ← factor 100 = centi-grados
```

---

## Hallazgo: los software sensors EXPIRAN

Si no se refrescan periódicamente (via escrituras al report 0x04), el firmware los marca como inválidos y los fans con curva apuntando a ellos caen al fallback.

### Observación empírica

- Escribir `virtual1 = 30.0°C` → el status report lo muestra inmediatamente.
- Esperar ~5-10s sin reescribir → el status empieza a mostrar `virtual1: None`.
- Durante la ventana de validez, los fans siguen la curva correctamente.

### Implicación práctica

El módulo NixOS necesita un **daemon que refresque continuamente los software sensors** (cada ~1-2s). Si muere, el fan caerá al fallback configurado en el `fan_setup` (típicamente 100% = seguro para los discos).

---

## Hallazgo: slots 8-15 no son usables en curvas

El status report expone 16 virtual sensor slots en `0x3C..0x5A`, pero sólo los slots **0-7** son utilizables por las curvas del firmware del QUADRO.

- Slots 0-7 (`virtual1..virtual8`): software sensors "de verdad". `fan.temp_sensor = 5..12` los lee.
- Slots 8-15 (`virtual9..virtual16`): aceptan escrituras y aparecen en el status, pero las curvas no los ven. El manual los llama "Virtual Software Sensors" y los describe como cálculos host-side de Aquasuite.

Probablemente el formato está compartido con el OCTO (que sí los tiene). En el QUADRO las posiciones existen pero son dead slots a efectos de curva.

---

## Hallazgo: interfaces USB separadas para HID y bulk

El QUADRO tiene **dos interfaces USB** (no sólo HID):

- Interface 0 (Vendor Specific): EP `0x02 OUT` bulk — por aquí se envía el report 0x04.
- Interface 1 (HID): reports 0x01, 0x02, 0x03, 0x08 vía `/dev/hidrawN`.

Intentar `HIDIOCSFEATURE` o `write()` con report 0x04 sobre hidraw falla con `EPIPE`. Esto despistó un buen rato porque los scripts de prueba iniciales usaban hidraw.

### Solución implementada

`src/device/linux.rs` ahora:
1. Busca el dispositivo USB por VID/PID escaneando `/sys/bus/usb/devices` (para obtener busnum/devnum).
2. Abre `/dev/bus/usb/BBB/DDD`.
3. Claim Interface 0 via `USBDEVFS_CLAIMINTERFACE` ioctl.
4. Escribe con `USBDEVFS_BULK` a EP `0x02`.
5. Release interface al drop.

Sin dependencias externas (`libc` nativo). Equivalente a lo que hace `libusb` pero ad-hoc.

### ⚠️ No hacer `detach_kernel_driver(0)`

Interface 0 es Vendor Specific — no tiene kernel driver asociado. Hacer detach devuelve éxito pero deja al dispositivo en estado inconsistente y requiere re-enumerar con:

```bash
echo 0 | sudo tee /sys/bus/usb/devices/3-4.2/authorized
echo 1 | sudo tee /sys/bus/usb/devices/3-4.2/authorized
```

Esto pasó una vez en medio de los tests, cuando el primer script pyusb llamó `detach_kernel_driver`. No volvió a pasar tras quitar esa llamada.

---

## Hallazgo: el firmware puede "corregir" puntos de curva fuera de rango

Al aplicar una curva con temps `[15, 17, 19, 21, 23, 25, 27, 30, 33, 36, 39, 42, 45, 48, 50, 55]°C` (en centi-grados = 1500, 1700, ..., 5500) observamos que el dispositivo almacenaba esto tal cual... pero cuando probamos con millicelsius (bug anterior), el firmware "corregía" puntos 8-15 a incrementos de +1 millicelsius (ej: 30001, 30002, 30003...).

Esto rompía la validación monotónica de la curva al leer de vuelta con `Temperature::from_millicelsius()` (que dividía por 10 perdiendo precisión, dando dos puntos iguales a `3000`).

**Lección**: el firmware valida la curva y mueve puntos para cumplir restricciones internas. Con unidades correctas (centi-grados) no hemos observado este comportamiento. Si vuelve a aparecer, revisar primero las unidades.

**Cambio preventivo**: `raw_report.rs::to_report()` ahora devuelve `Result<Report, QuadroError>` en vez de llamar `.expect()`, así un estado raro del dispositivo no paniquea.

---

## Lecciones aprendidas (para evitar ratholes futuros)

1. **El driver del kernel no soporta todo lo que el firmware puede hacer**. Si una limitación parece arbitraria (como `temp_sensor` ≤ 3), probablemente es del driver y no del firmware. Verificar empíricamente.

2. **No asumir unidades sin validar**. Nuestro error fue extrapolar de otros proyectos (Aquaero usa variantes, el kernel driver usa hwmon millidegrees en la API pero no en el wire). La única fuente fiable resultó ser leoratte/aquacomputer-quadro-control que sí tenía los factores en el código.

3. **Los software sensors expiran**. Ninguna implementación one-shot funciona; hace falta un daemon.

4. **Decompilar Aquasuite es una trampa**: el installer `.exe` es sólo un stub obfuscado que descarga el software real al ejecutarlo en Windows. El stub no contiene la lógica relevante. 30 min invertidos sin retorno.

5. **Capturar tráfico USB con Wireshark/USBPcap** es el approach correcto si hay que hacer más reverse engineering. No lo hicimos porque al final encontramos todo por deducción + leoratte, pero es la bala de plata para el futuro.

6. **pyusb + nix-shell es la forma más rápida de prototipar** en el NAS NixOS sin instalar nada permanente:

   ```bash
   sudo nix-shell -p python3Packages.pyusb python3Packages.crcmod --run "python3 <script>"
   ```
