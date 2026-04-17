# Referencias externas

## Kernel driver oficial

**Path local**: `/Users/danielramos/Documents/repos/others/aquacomputer_d5next-hwmon/aquacomputer_d5next.c`

**Upstream**: https://github.com/aleksamagicka/aquacomputer_d5next-hwmon

Este es el driver del kernel Linux. Soporta múltiples dispositivos Aquacomputer (D5 Next, Farbwerk, Octo, Quadro, Aquaero...). **Limitado**: sólo lee sensores y permite setear PWM manual (via hwmon). **NO** implementa curvas ni escritura de software sensors. Su `temp_select` está limitado a 0-3 (sólo físicos) — esto es una limitación del driver, NO del firmware.

Las hexpat docs en `re-docs/quadro/` son útiles para entender el layout del control report, aunque los offsets difieren ligeramente entre FW (el documentado en el repo es más viejo que nuestro FW 1033):

- `quadro_contol.hexpat` — layout del control report 0x03
- `quadro_sensors.hexpat` — layout del status report 0x01
- `quadro_virt_sensors.hexpat` — layout del report 0x04 (bulk write para virtual sensors)

Ver también `re-docs/PROTOCOLS.md` para overview.

## liquidctl

**Path local** (si sigue clonado): `/tmp/liquidctl`

**Upstream**: https://github.com/liquidctl/liquidctl

Doc específica: https://github.com/liquidctl/liquidctl/blob/main/docs/aquacomputer-quadro-guide.md

Proyecto Python maduro para control de hardware de cooling. Soporta QUADRO pero **sólo con PWM fijo**: no implementa curvas ni usa software sensors para curvas. Confirma implícitamente que el kernel driver tampoco lo hace.

Útil como referencia del pattern `aqc_get_ctrl_val` / `aqc_set_ctrl_val` y el mecanismo de envío del secondary/commit report tras cambios en el control report.

## leoratte/aquacomputer-quadro-control

**Path local** (si sigue clonado): `/tmp/aquacomputer-quadro-control`

**Upstream**: https://github.com/leoratte/aquacomputer-quadro-control

Proyecto Python con GUI, probado en FW 1028. **La clave decisiva** para descubrir que las temperaturas son centi-grados (×100) y no millicelsius (×1000).

Archivo crucial: `src/quadrocontrol/converter.py`:

```python
config.fans[i].curve_mode_vars.temp[x] = self.convert(2, 100)
config.fans[i].curve_mode_vars.percent[x] = self.convert(2, 100)
config.fans[i].curve_mode_vars.start_temp = self.convert(2, 100)
config.fans[i].temp_target_vars.temp_target = self.convert(2, 100)
config.temp_sensors[i] = self.convert(2, 100)
config.flow_sensor.correction_factor = self.convert(2, 100)
```

Factor 100 por todas partes = centi-grados / centi-percent.

**Importante**: el proyecto sólo lee/escribe el control report 0x03. No maneja software sensors (report 0x04) — eso lo descubrimos nosotros. Los offsets base del control report también difieren ligeramente entre FW 1028 y nuestro 1033; los del kernel driver son más confiables para FW 1033.

## Manual oficial del QUADRO

**URL**: https://aquacomputer.de/handbuecher.html?file=tl_files/aquacomputer/downloads/manuals/QUADRO_english.pdf

**Markdown convertido** (si sigue en `/tmp/`): `/tmp/quadro-manual/webfetch-1776450215331-s77qmf/webfetch-1776450215331-s77qmf.md`

Secciones clave:

- **9. Temperature sensors**
  - 9.1: 4 hardware sensors
  - 9.2: 1 flow sensor
  - 9.3: 8 software sensors ("the last eight sensors in the list")
- **10. Fan configuration**
  - 10.3 Fan mode curve controller — "up to nine available temperature sensors", descripción de `start_temp` behavior
  - 10.4 General fan settings — `min_percent`, `max_percent`, `fallback`, start boost
- **14. Playground (aquasuite)**
  - 14.2 Virtual Software Sensors — calculados host-side por Aquasuite, "may be transmitted via USB connection to connected devices that feature software sensors"

El "up to nine" del manual parece referirse a la UI de Aquasuite (que presenta los sensores seleccionables como un dropdown). En realidad son 13 índices utilizables (4+1+8), pero la UI condensa.

## Foros Aqua Computer

- Forum post sobre virtual sensors: https://forum.aquacomputer.de/weitere-foren/english-forum/113793-aquasuite-controlling-your-pumps-fans-with-virtual-sensors-and-automatic-condition-based-curve-switching/
- Forum post sobre QUADRO (general): https://forum.aquacomputer.de/weitere-foren/english-forum/109501-aquacomputer-quadro-fan-controller/

Ninguno de los posts que encontramos documenta el protocolo al nivel de bytes. Son más UI-oriented.

## Aquasuite (Windows, software oficial)

**Installer**: `~/Downloads/aquasuite_setup.exe` (1.2 MB, descargable de https://aqua-computer.de).

**Caveat importante**: es un **stub obfuscado (ConfuserEx)** que descarga el software real al ejecutarlo en Windows. El stub no contiene la lógica de control — sólo el downloader.

Decompilado (útil sólo para identificar namespaces): `/tmp/aquasuite-src/aquasuite_setup.decompiled.cs` (28k líneas, mayoría vacías por la ofuscación).

**No recomendamos gastar tiempo decompilando Aquasuite**. Lo intentamos durante 30 minutos y no dio retorno. Si se necesita más reverse engineering, capturar tráfico USB en vivo (Wireshark + USBPcap en Windows, o `usbmon` en Linux con el dispositivo pasado a una VM Windows) es mucho más productivo.

## Herramientas útiles

- **USBPcap** (Windows): https://desowin.org/usbpcap/ — captura de tráfico USB para Wireshark.
- **`usbmon`** (Linux): captura nativa. `modprobe usbmon`, luego `tcpdump -i usbmonN`.
- **pyusb**: API simple para libusb desde Python.
- **ilspycmd**: decompilador .NET CLI. `dotnet tool install -g ilspycmd`.
- **marker_single**: convierte PDF a markdown. Útil para el manual.

## Proyectos relacionados que no exploramos

- **fan2go** (Go): control de fans Linux genérico.
- **OpenRGB**: enfocado en iluminación, pero tiene soporte experimental para Aqua Computer.
- **fancontrol-gui** (KDE): GUI para hwmon, no toca el protocolo Aqua Computer.

Ninguno implementa curvas + software sensors para QUADRO hasta donde pudimos ver.
