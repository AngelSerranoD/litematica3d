# Edición de schematics con historial

<!-- Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Los schematics de la biblioteca se pueden modificar: cambiar el bloque de una
posición o sustituir todas las apariciones de un block state por otro. Cada
modificación guarda antes la versión que había, así que siempre se puede
deshacer (y rehacer). Todo ocurre en la copia de la biblioteca: el archivo
original del usuario no se toca nunca. Todo funciona sin red.

Código: `src-tauri/src/edicion.rs` (modelo y aplicación en memoria),
`src-tauri/src/biblioteca/historial.rs` (guardar, historial, restaurar,
exportar), `src-tauri/src/schematic/escritor_litematic.rs` (escritor) y
`src/api/edicion.ts` (funciones para el frontend).

## 1. Qué se modifica y en qué formato queda

- Al aplicar cambios se **sobrescribe la copia** de la biblioteca
  (`%APPDATA%\com.aserrano.litematica3d\biblioteca\<id>.<ext>`). La versión
  anterior pasa al historial.
- Lo modificado se guarda **siempre como `.litematic`**, aunque el original
  fuera `.schem`, `.schematic` o `.nbt`. Esa versión original sigue en el
  historial con su formato y se puede restaurar tal cual.
- Exportar escribe la copia actual como `.litematic` donde elija el usuario.

### Qué se pierde al convertir a .litematic

El escritor conserva bloques, paletas, posición y tamaño de cada región,
nombre, autor, descripción, fecha de creación y `MinecraftDataVersion`.

- **Original `.litematic`**: se conservan también las entidades de bloque
  (`TileEntities`: contenido de cofres, texto de carteles…), las entidades
  (`Entities`) y los ticks pendientes, tal cual venían. No se conservan la
  miniatura `PreviewImageData` ni `SubVersion` (Litematica las trata como
  opcionales).
- **Original `.schem`, `.schematic` o `.nbt`**: sus entidades de bloque y
  entidades **se pierden** (el lector no las lleva al modelo común); el
  `.litematic` sale con esas listas vacías. El original sigue en el historial.
- Donde un cambio sustituye el **tipo** de bloque (un cofre que pasa a ser
  piedra), se quitan la entidad de bloque y el tick pendiente de esa posición.
  Si solo cambian sus propiedades (girar el cofre), se conservan.
- Si el archivo no traía nombre (Sponge, MCEdit, estructuras), el `.litematic`
  lleva el nombre que tiene en la biblioteca.
- `MinecraftDataVersion`: la del original; si no tenía, 3955 (Minecraft 1.21.1,
  `VERSION_DATOS_POR_DEFECTO`). No se toma de la versión de texturas activa
  porque el núcleo no conoce su data version sin abrir el jar.

## 2. Modelo de cambios

Los block states van como el texto canónico del resto de la app, el mismo que
devuelve `bloque_en`: `minecraft:oak_stairs[facing=north,half=bottom]`. Se
normalizan antes de usarlos (`stone` → `minecraft:stone`, propiedades
ordenadas por clave) y se validan: namespace y nombre con minúsculas, cifras,
`_`, `-`, `.` (y `/` en el nombre); propiedades `clave=valor` con letras,
cifras, `_`, `-` o `.`, sin repetir; como mucho 512 caracteres. Si no, error
`edicionInvalida`.

```ts
interface Cambio {      // cambia el bloque de una posición
  region: string;       // nombre de la región (Region.nombre / BloqueSeleccionado.region)
  x: number; y: number; z: number; // coordenadas del SCHEMATIC (BloqueSeleccionado.posicion)
  nuevoEstado: string;  // "minecraft:air" para quitar el bloque
}
interface Reemplazo {   // cambia todas las apariciones de un estado exacto, en todas las regiones
  estadoOrigen: string;
  estadoDestino: string;
}
```

- Las coordenadas son las del schematic (las de `Region.origen` y las de
  `BloqueSeleccionado.posicion`), **no** las del modelo del visor (las de
  `malla` y `bloque_en`, que empiezan en 0): `posicion = origen del modelo +
  coordenadas del modelo`.
- Si hay varias regiones con el mismo nombre, se usa la última que contenga la
  posición. Región inexistente: `regionNoEncontrada`; posición fuera de la
  región: `edicionInvalida`.
- Orden: primero los reemplazos, en el orden de la lista (uno puede encadenar
  con el siguiente: A→B y luego B→C deja C); después los cambios puntuales, en
  orden (si dos tocan la misma posición, gana el último). Así un cambio puntual
  manda sobre un reemplazo en su posición.
- Todo se valida antes de tocar nada: un solo cambio malo rechaza la llamada
  entera y no se guarda nada.
- La paleta de cada región se recompacta: `minecraft:air` en el 0 y después
  solo los estados que se usan, sin repetidos.
- Los grupos (familias de bloques parecidos) se envían como varios
  `Reemplazo`, uno por estado; el motor de parecidos es otra pieza.

## 3. Comandos

Firmas Rust (`src-tauri/src/comandos.rs`) y TS (`src/api/edicion.ts`). Los
errores llegan como siempre: `{ tipo, mensaje }` con el mensaje en español.

### `aplicar_cambios(id, cambios, reemplazos) -> Edicion`

```rust
async fn aplicar_cambios(id: String, cambios: Vec<Cambio>, reemplazos: Vec<Reemplazo>) -> Result<Edicion, ErrorApp>
```
```ts
aplicarCambios(id: string, cambios: Cambio[], reemplazos?: Reemplazo[]): Promise<Edicion>
interface Edicion { entrada: Entrada; cambios: number; version: number | null }
```

Carga la copia actual, aplica, guarda la actual en el historial y escribe la
nueva copia `.litematic` (sección 5). `cambios` es el número de posiciones que
han cambiado de verdad; `version`, la del historial donde quedó lo de antes
(deshacer = `restaurarVersion(id, version)`). Si ninguna posición cambia (lista
vacía, o todo igual que estaba) no se toca nada: `cambios: 0`,
`version: null` y la entrada de antes.

Después de aplicar:

- El índice SQLite queda al día: formato, ruta (`<id>.litematic`), hash
  SHA-256, tamaño, recuentos, dimensiones, `version_datos`.
- La caché del visor (`CacheVisor`) olvida el id: la próxima `malla` o
  `bloque_en` vuelve a leer el archivo. Los materiales no tienen caché en el
  núcleo (se calculan de la copia en cada llamada): basta con volver a
  pedirlos.
- La miniatura queda **obsoleta** (`miniatura_texturas = NULL`, ver
  docs/miniaturas.md): el PNG viejo se sigue viendo y la cola de miniaturas la
  regenera en cuanto el frontend pone la `entrada` devuelta en
  `useBiblioteca`.

Errores: `noEncontrado`, `archivoPerdido`, `archivoCorrupto`,
`regionNoEncontrada`, `edicionInvalida`, `io`, `baseDatos`.

### `historial(id) -> Version[]`

```rust
async fn historial(id: String) -> Result<Vec<Version>, ErrorApp>
```
```ts
historial(id: string): Promise<Version[]>
interface Version {
  version: number;      // identificador (marca en ms, única en el schematic)
  fecha: number;        // ms: cuándo dejó de ser la actual
  formato: Formato;     // formato del archivo guardado
  motivo: "aplicar" | "restaurar"; // qué la sustituyó
  cambios: number;      // posiciones que cambió esa operación (0 al restaurar)
  totalBloques: number;
  tamanoArchivo: number;
}
```

De la más reciente a la más antigua; como mucho 20. Error `noEncontrado` si el
id no existe.

### `restaurar_version(id, version) -> Edicion`

```rust
async fn restaurar_version(id: String, version: i64) -> Result<Edicion, ErrorApp>
```
```ts
restaurarVersion(id: string, version: number): Promise<Edicion>
```

La actual pasa antes al historial (motivo `restaurar`), así que **rehacer** es
restaurar esa versión (la más reciente del historial). La restaurada se copia
tal cual, con su formato (puede volver a ser `.schem`), y sigue también en el
historial. Índice, caché del visor y miniatura se actualizan igual que al
aplicar. `cambios` es 0. Errores: `versionNoEncontrada` (no existe o ya se
podó), `archivoPerdido`, `archivoCorrupto`, `io`, `baseDatos`.

### `exportar_schematic(id, ruta) -> null`

```rust
async fn exportar_schematic(id: String, ruta: String) -> Result<(), ErrorApp>
```
```ts
exportarSchematic(id: string, ruta: string): Promise<null>
elegirDestinoLitematic(nombrePropuesto: string): Promise<string | null> // src/api/dialogo.ts
```

Escribe la copia actual en `ruta`, elegida con el diálogo de guardar. Igual
que `guardar_texto`, solo acepta rutas absolutas que acaben en `.litematic`
(sin distinguir mayúsculas); si no, `nombreInvalido` y no escribe nada. Si la
copia es `.litematic` sale tal cual; si no, se convierte con el escritor
(sección 1). Escribe en `<ruta>.tmp` y lo renombra encima: si falla, el
archivo que hubiera en `ruta` sigue entero. No toca la biblioteca.

## 4. Historial en disco

- Archivos: `<datos>/biblioteca/historial/<id>/<versión>.<ext>`, con `<ext>`
  el de esa versión (`litematic`, `schem`, `schematic`, `nbt`). El protocolo
  `asset` no llega a esa carpeta (solo a las miniaturas).
- Índice: tabla `historial`, añadida en el **esquema v4** (`PRAGMA
  user_version = 4`; migra desde v1, v2 y v3 al abrir):

```sql
CREATE TABLE historial (
    schematic_id TEXT NOT NULL REFERENCES schematics (id) ON DELETE CASCADE,
    version INTEGER NOT NULL,        -- marca en ms; da nombre al archivo
    fecha INTEGER NOT NULL,          -- ms: cuándo se archivó
    formato TEXT NOT NULL,           -- litematic | sponge | mcedit | estructura
    ruta TEXT NOT NULL,              -- relativa a la carpeta biblioteca
    motivo TEXT NOT NULL,            -- aplicar | restaurar
    cambios INTEGER NOT NULL,
    total_bloques INTEGER NOT NULL,
    tamano_archivo INTEGER NOT NULL,
    PRIMARY KEY (schematic_id, version)
) WITHOUT ROWID;
```

- `version` es la hora en ms, o una más que la última del mismo schematic si
  el reloj no ha avanzado; nunca pisa un archivo que ya exista.
- **Límite: 20 versiones** por schematic (`MAX_VERSIONES`). Al pasar de 20,
  las más antiguas van a la **Papelera** y se quitan sus filas (solo si la
  Papelera las acepta; si falla, la edición sale bien igual y se vuelve a
  intentar en la siguiente).
- **Eliminar** un schematic manda a la Papelera, en una sola operación, la
  copia, la miniatura y su carpeta de historial, y quita sus filas
  (`schematics`, `comprobacion`, `historial`).
- La Papelera está detrás del trait `Papelera` (`PapeleraSistema` usa la crate
  `trash`); las pruebas usan una simulada con `Biblioteca::abrir_con_papelera`.
- Si al restaurar el contenido coincide byte a byte con el de otra entrada (se
  editó A, se volvió a importar el original y se restaura el original de A),
  el hash de A se guarda como `<hash>:<id>` para no romper la unicidad; la
  importación sigue detectando el duplicado por la otra entrada.

## 5. Atomicidad

Una sola edición a la vez por biblioteca (aplicar, restaurar y eliminar
comparten un candado). Para sustituir la copia:

1. Lo nuevo se escribe en `<id>.edicion.tmp`, en la misma carpeta, con
   `sync_all`. Si falla, se quita el temporal y no ha cambiado nada.
2. **Misma extensión** (lo normal: `.litematic` → `.litematic`): la copia
   actual se **copia** al historial (por un temporal, para que solo exista
   entera) y el temporal se **renombra encima** de la copia. La copia
   anterior no deja de existir en ningún momento.
   **Otra extensión** (`.schem` → `.litematic`, o al restaurar un original):
   el temporal pasa a `<id>.<ext nueva>` y la copia anterior se **mueve** al
   historial (mismo disco: es un renombrado).
3. Las filas (`INSERT` en `historial` y `UPDATE` de `schematics`) van en una
   transacción que solo se confirma cuando los archivos ya están en su sitio.
   Si algo falla antes, se deshace lo hecho (se quitan el temporal y la copia
   al historial recién creada, o se devuelve el archivo movido) y la copia
   anterior y el índice quedan como estaban. Si falla la confirmación, la
   versión archivada vuelve a su sitio.

En Windows el renombrado se reintenta unas veces (el visor o el webview
pueden tener el archivo abierto un instante), como con las miniaturas.

## 6. Límites

| Qué | Máximo |
| --- | --- |
| Versiones en el historial por schematic | 20 (`MAX_VERSIONES`) |
| Cambios puntuales por llamada | 1 000 000 (`MAX_CAMBIOS`) |
| Reemplazos por llamada | 10 000 (`MAX_REEMPLAZOS`) |
| Longitud de un block state | 512 caracteres |

## 7. Para la interfaz

- Tras `aplicarCambios` o `restaurarVersion`, poner `edicion.entrada` en
  `useBiblioteca` (la cola de miniaturas ve `texturas: null` y la regenera),
  volver a pedir la malla del visor y, si está abierta, la lista de materiales.
- Deshacer = `restaurarVersion(id, edicion.version)`; rehacer =
  `restaurarVersion(id, historial[0].version)` justo después de deshacer.
- Para exportar: `elegirDestinoLitematic(entrada.nombre)` y
  `exportarSchematic(id, ruta)`.
