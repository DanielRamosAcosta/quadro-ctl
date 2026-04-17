# Mapeo crítico: `fan.temp_sensor` → sensor

> Este documento captura el hallazgo más difícil de esta investigación. Léelo antes de tocar nada relacionado con curvas.

## Tabla de mapeo

El campo `temp_sensor` (u16 BE en `fan_base + 0x03` del control report) selecciona qué sensor gobierna la curva:

| Valor | Sensor | Dónde se lee el valor actual |
|---|---|---|
| 0 | Físico 1 (sensor1) | Status report `0x34` |
| 1 | Físico 2 (sensor2) | Status report `0x36` |
| 2 | Físico 3 (sensor3) | Status report `0x38` |
| 3 | Físico 4 (sensor4) | Status report `0x3A` |
| 4 | **Flow sensor** | Status report `0x6E` |
| **5** | **Software sensor 1** | Status report `0x3C` (slot 0, aka `virtual1`) |
| 6 | Software sensor 2 | `0x3E` (slot 1 / `virtual2`) |
| 7 | Software sensor 3 | `0x40` (slot 2) |
| 8 | Software sensor 4 | `0x42` (slot 3) |
| 9 | Software sensor 5 | `0x44` (slot 4) |
| 10 | Software sensor 6 | `0x46` (slot 5) |
| 11 | Software sensor 7 | `0x48` (slot 6) |
| 12 | Software sensor 8 | `0x4A` (slot 7) |
| 13-19 | **Inválido** — fan usa fallback | — |
| 65535 (0xFFFF) | "no sensor" — fan usa fallback | — |

## Por qué el kernel driver se queda corto

El driver del kernel `aquacomputer_d5next.c` limita `temp_sensor` a 0-3 (sólo físicos). Es una **limitación del driver, NO del firmware**. El firmware acepta perfectamente 0-12; simplemente aquasuite es la única app que lo explota.

Esto despistó durante toda la sesión: si el kernel driver no lo soporta, uno asume que no se puede. La verdad es que sí se puede — sólo hay que escribir los valores correctos.

## El quiebre: slots 0-7 vs 8-15

El status report expone 16 virtual sensor slots en `0x3C..0x5A`. Pero **sólo los slots 0-7 son usables como software sensors en fan curves**:

- **Slots 0-7** (aka `virtual1..virtual8`): software sensors escribibles desde host via report 0x04. Usables en curvas.
- **Slots 8-15** (aka `virtual9..virtual16`): reservados para los "Virtual Software Sensors" que Aquasuite calcula host-side. Aparecen en el status report porque el formato es compartido con OCTO (que sí los tiene), pero en el QUADRO no son usables por las curvas del firmware.

## Cómo se descubrió

Después de ~50 pruebas fallidas con distintos valores de `temp_sensor`, un script iteró sistemáticamente `(temp_sensor ∈ 5..12, slot ∈ 0..15, valor_temp ∈ {15°C, 50°C})` sustaining writes via bulk. Match limpio:

```
TS=5  SLOT=0 delta=6413
TS=6  SLOT=1 delta=6413
TS=7  SLOT=2 delta=6413
TS=8  SLOT=3 delta=6413
TS=9  SLOT=4 delta=6413
TS=10 SLOT=5 delta=6413
TS=11 SLOT=6 delta=6413
TS=12 SLOT=7 delta=6413
```

El match TS=N ↔ SLOT=N-5 con la MISMA magnitud de respuesta (6413 centi-percent) confirma la correspondencia 1:1.

**Nota importante**: este descubrimiento sólo fue posible **después** de arreglar el bug de centi-grados en las curvas (ver [03-findings-and-bugs.md](./03-findings-and-bugs.md)). Con curvas en millicelsius, el firmware interpretaba los puntos como 200°C+ y el fan siempre se quedaba en el primer punto, ocultando cualquier variación por sensor.

## Validación cruzada con el manual oficial

El manual del QUADRO (`Manual_QUADRO_english.pdf`, sección 9) describe la lista de sensores para curvas:

- **9.1** "The first four sensors in the list represent the temperature sensor inputs of the QUADRO." → índices 0-3.
- **9.2** "The fifth sensor in the list represents the flow sensor input." → índice 4.
- **9.3** "The last eight sensors in the list are software sensors and can be used to transmit sensor data that is not physically available to the QUADRO controller from the computer by USB connection." → índices 5-12.

Sección 10.3: "This mode can be used to automatically adjust fan speed depending on the current temperature reading of **one of the up to nine available temperature sensors**."

Los "nine" del manual no cuadran con 13 (4+1+8). Probablemente la UI de Aquasuite muestra 9 opciones (tal vez un hw + flow + 8 software, o similar — el detalle no es relevante porque empíricamente 0-12 funcionan todos).

## Cómo usar desde Rust (ya implementado)

```json
// Config para fan1 con software sensor 1
{
  "fans": {
    "fan1": {
      "mode": "curve",
      "sensor": 5,
      "points": [
        {"temp": 20.0, "percentage": 20},
        {"temp": 25.0, "percentage": 30},
        ...
      ]
    }
  }
}
```

Y escribir el valor del software sensor 1:

```json
// Config para sensors set
{
  "virtual1": 45.0
}
```

El comando `quadro-ctl sensors set --config-file ...` construye el report 0x04 y lo envía via bulk. `virtual1` mapea a slot 0, que es lo que `fan.temp_sensor = 5` lee.
