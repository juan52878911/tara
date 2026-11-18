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

> Estado: M0 implementado. Diseño en [docs/diseno.md](docs/diseno.md); contrato en
> [schema/tara.v1.json](schema/tara.v1.json).

```bash
cargo build --offline

tara detect [ruta] [--json|--compact]   # perfil: lenguajes, frameworks, ORM, bundler, IaC, docs
tara scan [ruta]                        # informe JSON; escribe .tara/baselines/<sha>.json y .tara/baselines/latest
tara compare <sha|ref|latest> [ruta] [--threshold-pct N] [--json|--compact]
```

- `scan` ejecuta los adaptadores (por ahora `loc`: líneas por fichero de código,
  dimensión `code_size`). Hallazgos ordenados por `id`; con `TARA_NOW` fijado, dos `scan`
  del mismo commit producen ficheros idénticos.
- `compare` escanea el árbol actual y lo empareja con la línea base por
  `(dimension, metric, location)`: delta absoluto y %. Umbral por defecto 5 %.
  Códigos de salida: `0` sin regresión, `1` algún valor sube más del umbral, `2` falta
  la línea base o error de uso.
- Un adaptador cuyo binario externo falta aparece en `coverage` como `skipped` con su `reason`.

## Licencia

[Apache-2.0](LICENSE).
