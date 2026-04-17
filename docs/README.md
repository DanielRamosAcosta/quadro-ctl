# Documentación — QUADRO reverse engineering

Esta carpeta recoge todo lo investigado sobre el protocolo USB del Aqua Computer QUADRO y los hallazgos necesarios para usar los **software sensors** como fuente de las curvas de fans.

## Índice

1. **[01-protocol.md](./01-protocol.md)** — Protocolo USB: HID reports, interfaces, estructura de cada report.
2. **[02-sensor-mapping.md](./02-sensor-mapping.md)** — El mapeo crítico `fan.temp_sensor` → sensor. **Lectura imprescindible**.
3. **[03-findings-and-bugs.md](./03-findings-and-bugs.md)** — Bugs descubiertos (centi-grados vs millicelsius), expiración de virtual sensors, lecciones aprendidas.
4. **[04-next-steps.md](./04-next-steps.md)** — Trabajo pendiente: watch daemon, módulo NixOS, validación end-to-end.
5. **[05-testing.md](./05-testing.md)** — Scripts de test disponibles en el NAS, flujo de deploy.
6. **[06-references.md](./06-references.md)** — Proyectos externos, manual oficial, capturas USB.

## TL;DR

El QUADRO soporta curvas de fans basadas en **software sensors** (sensores escritos desde el host via USB). La documentación se requirió para descubrir dos cosas no obvias:

- **Mapeo de índices**: `fan.temp_sensor = 5..12` → software sensors 1..8 (mapeados a slots 0-7 del array virtual en el status report). Ver [02-sensor-mapping.md](./02-sensor-mapping.md).
- **Unidad de la curva**: las temperaturas en la curva son **centi-grados (×100)**, NO millicelsius (×1000). Ver [03-findings-and-bugs.md](./03-findings-and-bugs.md).

El código ya está refactorizado. Queda el daemon periódico de refresco de virtual sensors, validación en el NAS y la integración con el módulo NixOS. Ver [04-next-steps.md](./04-next-steps.md).
