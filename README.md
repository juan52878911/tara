# tara

Estima el consumo de recursos de un proyecto (CPU, memoria, base de datos, tamaño de bundle,
coste de nube) **sin ejecutarlo, o ejecutando lo mínimo y de forma determinista**, prioriza
dónde mirar y guarda una **línea base por commit** para medir cada optimización o
refactorización.

- Cada cifra lleva su método y su confianza: lo que no se puede estimar, se dice.
- No analiza todo el proyecto: prioriza por historia del código (con
  chrono si está, con `git` si no) y analiza a fondo solo el top N.
- Núcleo agnóstico con contrato JSON versionado; integraciones (Claude Code, MCP, CI,
  arneses de construcción) como capas finas y opcionales.

> Estado: diseño. Ver [docs/diseno.md](docs/diseno.md).

```bash
tara scan              # perfil + priorización + hallazgos + línea base del commit actual
tara compare main      # deltas contra la línea base de otra ref
tara report            # oportunidades ordenadas por impacto × confianza
```

## Licencia

[Apache-2.0](LICENSE).
