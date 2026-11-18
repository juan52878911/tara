# tara — diseño

> Estado: borrador de diseño (v0). M0 implementado (`detect`, `scan`, `compare`, adaptador `loc`).

## 1. Problema

Optimizar o refactorizar un proyecto exige saber **dónde** pesa y **cuánto**, y hoy eso se
averigua de dos formas caras: ejecutándolo con carga (lento, ruidoso, necesita entorno) o
leyéndolo entero (no escala). `tara` estima el consumo de recursos de un proyecto **sin
ejecutarlo, o ejecutando lo mínimo y de forma determinista**, prioriza dónde mirar y guarda una
**línea base por commit** contra la que medir cada mejora.

## 2. Principios

1. **Cada cifra dice cómo se obtuvo y cuánto fiarse.** Ninguna estimación sin `method` y
   `confidence`. Lo que no se puede estimar se declara como tal, no se inventa.
2. **No analizar todo.** Primero se prioriza (historia del código, tamaño, arreglos
   acumulados) y solo el top N pasa por los análisis caros.
3. **Determinista por defecto.** Misma entrada y mismo commit → misma salida. Lo no
   determinista (LLM, `EXPLAIN` contra una base viva) se marca y se ancla aparte.
4. **Núcleo agnóstico, integraciones finas.** El núcleo es un CLI con un contrato JSON
   versionado; no conoce ningún arnés, editor ni proveedor de LLM. Todo lo demás son
   adaptadores opcionales.
5. **Degradar, no fallar.** Si falta una herramienta externa (chrono, infracost, semgrep…), el
   adaptador se salta y el informe dice qué dimensión quedó sin cubrir.

## 3. Qué se puede estimar y con qué confianza

| Dimensión | Fuente | Método | Confianza |
|---|---|---|---|
| `bundle` (tamaño de front) | metafile de esbuild/Vite/webpack, size-limit | `declared` (build real) | alta |
| `cloud_cost` | Terraform, manifiestos K8s/Helm (requests/limits) vía Infracost | `declared` | alta en recursos fijos; baja en lo que depende de uso (egress, peticiones, serverless) |
| `cpu` | conteo de instrucciones (callgrind / iai-callgrind / CodSpeed) sobre benchmarks existentes | `simulated` (ejecuta, determinista) | alta para comparar versiones; no da tiempo absoluto |
| `db` | reglas estáticas (consulta ORM dentro de bucle, N+1, `SELECT *` sin límite) + `EXPLAIN` sin `ANALYZE` opcional | `static` / `planner` | media; `EXPLAIN` solo compara planes y engaña con estadísticas vacías |
| `io` / `mem` | patrones estáticos (cargar todo en memoria, lecturas en bucle, colecciones sin cota) | `static` | baja-media: marca sospechas |
| `code_size` (contexto) | líneas por fichero de código (adaptador interno `loc`) | `static` | alta; no es consumo, es el peso para priorizar |
| `carbon` | derivado de `cloud_cost`/`cpu` con factores públicos | `model` | orden de magnitud |
| sospechas generales | revisor LLM **solo sobre hotspots** | `llm_suspect` | baja: dice dónde mirar, nunca una cifra |

Descartado como predictor de consumo: complejidad ciclomática/cognitiva (mide mantenibilidad,
no recursos). Se puede adjuntar como contexto de un hallazgo, nunca como hallazgo.

## 4. Arquitectura

```
tara scan
  1. detect     → perfil del proyecto (lenguajes, frameworks, ORM, bundler, IaC, docs)
  2. budgets    → presupuestos declarados en la documentación (p95, coste máx., tamaño máx.)
  3. prioritize → ranking de entidades (fichero/función) por peso × cambio × arreglos
  4. analyze    → adaptadores por dimensión, sobre el perfil y el top N
  5. snapshot   → línea base JSON anclada a sha
tara compare <sha|ref>  → deltas contra una línea base
tara report             → oportunidades ordenadas (impacto × confianza × peso del hotspot)
```

### 4.1 Detección (`detect`)

Ficheros de manifiesto y convenciones: `Cargo.toml`, `package.json`, `pyproject.toml`,
`go.mod`, `pom.xml`/`build.gradle`, `*.tf`, `Chart.yaml`, `kustomization.yaml`, manifiestos K8s,
config de bundler, ORM declarado en dependencias. Sale un `profile.json` que decide qué
adaptadores aplican.

### 4.2 Presupuestos desde la documentación (`budgets`)

Lee ADR, escenarios de atributos de calidad, PRD o un `tara.toml` explícito y extrae
presupuestos con unidad (`p95 < 300 ms`, `coste < 200 USD/mes`, `bundle < 250 kB gz`). La
extracción heurística (regex sobre unidades) es determinista; la extracción con LLM es
opcional y se marca. El informe compara estimado contra presupuesto.

### 4.3 Priorización (`prioritize`)

Interfaz `HistoryProvider` con dos implementaciones:

- **chrono** (si hay `chrono` y `.chrono/index.db`): `hotspots`, `churn`, `bugs`, `coupling`
  en JSON. Mejor ranking, con historia por función si el índice tiene `--symbols`.
- **git** (fallback): `git log --numstat` para churn y mensajes tipo `fix` para arreglos.
  Peor ranking, mismo contrato.

`score = peso (LOC/tamaño) × cambio reciente × densidad de arreglos`, normalizado. El top N
(configurable, por defecto 20) es lo único que reciben los adaptadores caros y el LLM.

### 4.4 Adaptadores (`analyze`)

Trait común:

```rust
trait Adapter {
    fn id(&self) -> &'static str;
    fn applies(&self, profile: &Profile) -> bool;
    fn requires(&self) -> &[ExternalTool];          // binarios externos; si faltan → skipped
    fn run(&self, ctx: &Ctx) -> Result<Vec<Finding>>;
}
```

Primeros adaptadores, por orden de valor/esfuerzo:

1. `bundle-esbuild` / `bundle-size-limit`
2. `iac-infracost` (Terraform y K8s)
3. `static-rules` (reglas propias con tree-sitter o Semgrep: ORM en bucle, I/O en bucle,
   carga completa en memoria) — por lenguaje
4. `cpu-callgrind` (opcional, ejecuta benchmarks existentes)
5. `db-explain` (opcional, necesita una base con estadísticas realistas)
6. `llm-suspects` (opcional, proveedor configurable, solo top N)

Los adaptadores externos se pueden escribir en cualquier lenguaje: `tara` ejecuta un binario
`tara-adapter-<id>` que recibe `Ctx` por stdin y devuelve `Finding[]` por stdout (mismo
contrato JSON).

### 4.5 Línea base (`snapshot` / `compare`)

- Fichero: `.tara/baselines/<sha>.json` (+ `latest` apuntando al último).
- Dos clases de evidencia, como en cualquier pipeline de evidencia serio:
  - **determinista** (`declared`, `static`, `simulated`): se re-ancla al commit actual;
  - **viva** (`planner` contra base real, `llm_suspect`): se ancla al último commit que tocó
    las rutas medidas (`git log -1 -- <rutas>`) y solo se regenera cuando ese código cambia.
- `compare` empareja hallazgos por `(dimension, metric, location)` y da delta absoluto y %,
  con umbral de ruido por método (instrucciones ±0,1 %, bundle ±0 bytes, coste ±0).
- Con chrono, cada snapshot se puede ingestar (`chrono add cmd:tara`) y se obtiene la serie
  temporal y «qué commit subió X» sin que `tara` mantenga una base de datos propia.

## 5. Contrato JSON (v1)

```json
{
  "schema_version": 1,
  "tool": "tara",
  "commit": "abc123",
  "generated_at": "2026-11-18T00:00:00Z",
  "profile": { "languages": ["rust"], "iac": ["terraform"], "bundler": null },
  "coverage": [
    { "dimension": "cloud_cost", "adapter": "iac-infracost", "status": "ok" },
    { "dimension": "cpu", "adapter": "cpu-callgrind", "status": "skipped", "reason": "valgrind no instalado" }
  ],
  "findings": [
    {
      "id": "cloud_cost:aws_db_instance.main",
      "dimension": "cloud_cost",
      "metric": "monthly_usd",
      "value": 182.4,
      "unit": "USD/month",
      "method": "declared",
      "confidence": "high",
      "location": { "path": "infra/db.tf", "line": 12, "symbol": "aws_db_instance.main" },
      "anchored_at": { "sha": "abc123", "class": "deterministic" },
      "budget": { "limit": 150, "source": "docs/adr/0003-coste.md" },
      "hotspot_score": 0.71,
      "evidence": "infracost breakdown --format json"
    }
  ]
}
```

`method ∈ {declared, static, simulated, planner, model, llm_suspect}` ·
`confidence ∈ {high, medium, low}` · `class ∈ {deterministic, live}`.

Las salidas usan `--json` (por defecto) y `--compact` (tabla de texto para agentes; en `detect` y `compare`). `TARA_NOW` fija `generated_at` para salidas byte-idénticas. Códigos de
salida: `0` ok, `1` regresión sobre umbral en `compare`, `2` error de uso, `3` cobertura
parcial (algún adaptador saltado) cuando se pide `--strict`.

## 6. Integraciones (fuera del núcleo)

- **Plugin de Claude Code**: skill «dónde optimizar» + comandos `scan`/`compare`; nunca
  reimplementa lógica, solo llama al CLI.
- **MCP** opcional que expone `scan`, `compare`, `report`.
- **Arneses de construcción / CI**: un adaptador del lado del arnés lee el JSON y lo convierte
  en su formato de evidencia (por ejemplo, un veredicto de release solo informativo). Vive en
  el arnés, no aquí.
- **chrono**: opcional en todo momento.
- **LLM**: proveedor configurable en `tara.toml` (o ninguno). Sin LLM, `tara` funciona entero
  salvo `llm-suspects`.

## 7. Configuración (`tara.toml`)

```toml
[prioritize]
provider = "auto"        # auto | chrono | git
top_n = 20

[adapters]
enabled = ["bundle-esbuild", "iac-infracost", "static-rules"]

[llm]
provider = "none"        # none | anthropic | openai-compatible | command
# command = "mi-llm --json"

[budgets]
"bundle.main_gz_kb" = 250
"cloud_cost.monthly_usd" = 300
```

## 8. Fuera de alcance (v0)

- Medir latencia o throughput reales bajo carga (eso es k6/Gatling; `tara` puede leer sus
  resultados como evidencia viva, no generarlos).
- Profiling en producción (OpenCost, Pyroscope): mismo criterio, se pueden importar.

## 9. Plan por hitos

1. **M0** — CLI (`clap`), `detect`, contrato JSON v1, `snapshot`/`compare` con un adaptador
   de prueba. Tests de contrato.
2. **M1** — `prioritize` con proveedores git y chrono.
3. **M2** — `bundle-esbuild` e `iac-infracost`.
4. **M3** — `static-rules` con tree-sitter para un primer lenguaje (TS o Python) y protocolo
   de adaptadores externos.
5. **M4** — `cpu-callgrind`, `budgets` desde docs y `report`.
6. **M5** — plugin de Claude Code y MCP.

## 10. Referencias

- BigO(Bench), Meta/Inria 2025 — los LLM aciertan ~64 % en complejidad temporal: <https://arxiv.org/abs/2503.15242>
- Infracost para Kubernetes: <https://www.infracost.io/docs/features/kubernetes/>
- Benchmarks consistentes en CI con conteo de instrucciones: <https://pythonspeed.com/articles/consistent-benchmarking-in-ci>
- iai-callgrind: <https://iai-callgrind.github.io/iai-callgrind/latest/html/index.html>
- size-limit: <https://github.com/ai/size-limit>
