# Edición de schematics con historial

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

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
  (`%APPDATA%\com.aserrano.schematiclab\biblioteca\<id>.<ext>`). La versión
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

Firmas Rust (`src-tauri/src/comandos/edicion.rs`) y TS (`src/api/edicion.ts`). Los
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

### `exportar_schematic(id, nombre) -> string | null`

```rust
async fn exportar_schematic(id: String, nombre: String) -> Result<Option<String>, ErrorApp>
```
```ts
exportarSchematic(id: string, nombre: string): Promise<string | null>
```

Abre desde Rust el diálogo de guardar, con `nombre` como propuesta, y escribe
la copia actual en la ruta elegida (si le falta `.litematic`, se le añade).
Devuelve esa ruta, o `null` si el usuario cancela. El frontend nunca manda la
ruta: así un webview comprometido no puede escribir donde quiera. Aun así, la
biblioteca solo acepta rutas absolutas que acaben en `.litematic` (sin
distinguir mayúsculas); si no, `nombreInvalido` y no escribe nada. Tampoco
deja exportar dentro de la carpeta de copias de la biblioteca (pisaría la copia
de otro schematic): también `nombreInvalido`. Si la
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
- Para exportar: `exportarSchematic(id, entrada.nombre)` (el diálogo lo abre
  Rust).
- Iconos de bloques que no están en el schematic (la lista de parecidos) y
  previsualización: `celdas_atlas(estados) -> { atlas, celdas } | null`
  (`src/api/atlas.ts`, como mucho 256 estados por llamada). Una fila por
  estado con sus 7 caras en el orden de `Cara` (arriba, abajo, norte, sur,
  oeste, este, cruz): `{ celda, rotacion, color }` o `null`. `null` entero si
  el atlas aún no está listo o no hay jar (no genera nada); la `version` de
  `atlas` tiene que casar con la del atlas cargado en el frontend.
- Sin atlas (sin jar), el color plano de cualquier estado:
  `colores_bloques(estados) -> ([r, g, b] | null)[]` (el `color_de` de la
  malla; como mucho 256 por llamada).
- Previsualización de los pendientes en el visor, sin remallar:
  - **Reemplazos**: se reescriben las tablas por material del shader (color,
    celda del atlas y giro) de los materiales cuyo estado visual es el origen
    (`claveVisual` en `src/api/visor.ts`, igual que `clave_visual` en Rust),
    con las caras del destino; misma forma, mismos quads. Las dos mallas (la
    completa y la del corte) tienen su tabla y se reescriben las dos.
  - **Cambios sueltos**: una capa encima (`src/componentes/visor/previaEdicion.ts`)
    con las caras de la celda copiadas de su chunk (recortadas si el greedy
    meshing las fusionó) y las texturas del destino, más un marco de alambre.
  - **Resaltados**: la familia seleccionada lleva una trama (canal verde de la
    tabla de giros) y el bloque pinchado un tinte (un uniform con su celda).
- Estado de la interfaz: `src/estado/edicion/` (un almacén armado con trozos:
  modo Visualizar/Modificar y confirmaciones, bloque pinchado y lista de
  parecidos, pendientes, familia, historial y celdas de los iconos). Su panel
  está en `src/componentes/edicion/` (un archivo por sección).

## 8. Parecidos

Sugerencias para cambiar un bloque por otro de la misma forma y color
parecido, o una familia de material entera por otra (toda la pizarra
profunda por piedra negra). No cambian nada: devuelven estados y reemplazos
que se aplican con `aplicar_cambios`. Todo funciona sin red y sin jar.

Código: `src-tauri/src/parecidos/` (`color.rs`, `forma.rs`, `propiedades.rs`,
`exclusiones.rs`, `familias.rs`, `sugerir.rs` y el `Catalogo` en `mod.rs`) y
`src/api/parecidos.ts`.

### Catálogo

Los bloques conocidos de la versión de texturas elegida: los
`blockstates/*.json` del jar, con la forma, las propiedades y el color de
cada uno. Sin jar, la tabla integrada `datos/colores_bloques.json` más los
muros vanilla, con colores planos. Los comandos esperan a que el atlas esté
preparado y guardan el catálogo en memoria (`CacheCatalogo`) hasta que
cambia la versión o el jar.

### Color

- Color de un block state = media de las texturas de sus caras en ese
  estado: la de **arriba cuenta ×2** y cada lateral ×1. Así un tronco
  `axis=y` se parece a sus anillos y uno `axis=x` a su corteza.
- Los píxeles con **alfa < 128** no cuentan (cristales, hojas, rejillas); si
  un estado no tiene ninguna cara opaca, se usa el color del bloque.
- Las texturas tintadas (hierba, hojas…) llevan el tinte del atlas.
- Se compara con la distancia euclídea en **OKLab** (ΔE_ok): por debajo de
  ~0,02 apenas se nota; por encima de ~0,1 son colores distintos.
- Las medias por textura y el color de cada bloque se guardan en
  `<cache>/texturas/<versión>/colores.json`, junto al atlas; se recalculan
  si cambia el jar o el esquema del archivo.

### Formas

`entero`, `losa`, `escalera`, `muro`, `valla`, `puertaValla`, `puerta`,
`trampilla`, `boton`, `placaPresion`, `panel` (paneles y barrotes),
`alfombra` y `otro`. Se deducen del sufijo del id y, si no lo tiene, de si es
un cubo completo (con jar, por sus modelos; sin jar, por cómo lo dibuja el
visor). **Solo se sugieren bloques de la misma forma**: una losa nunca
sugiere un bloque entero ni una escalera una losa.

Los de forma `otro` (velas, antorchas, brotes, faroles…) solo sugieren
bloques con su mismo «id base»: el id sin tinte, especie de madera, grado de
oxidación del cobre, `waxed_` ni `soul_` delante (`white_candle` →
`red_candle`; `torch` → `soul_torch`; `oak_sapling` → `birch_sapling`). Si no
hay ninguno, nada: cambiar un «otro» por cualquier otro de color parecido
rara vez tiene sentido (un farol por una flor).

### Propiedades del estado destino

El estado destino conserva las propiedades del origen que el destino admite
con ese valor (`facing`, `half`, `shape`, `type`, `axis`, `waterlogged`,
`north`/`up` de muros y vallas…); las que no tiene se descartan y las que le
faltan toman su valor por defecto (`facing=north`, `half=bottom`,
`type=bottom`, `shape=straight`, `axis=y`, `waterlogged=false`, `up=true` en
muros). Con jar las propiedades salen del blockstate; sin jar, de una
plantilla por forma (más `axis` en troncos, madera, tallos, pilares,
basalto, balas de heno…, para no girarlos).

### Exclusiones

Nunca se sugieren como destino:

- Aire y fluidos.
- Bloques técnicos (comandos, estructuras, barreras, luz…).
- Bloques sin objeto o duplicados visuales: macetas con planta, antorchas y
  carteles de pared, partes de plantas, calderos llenos… y los
  **infestados** (idénticos a su bloque normal).
- Bloques **con datos** (cofres, barriles, hornos, tolvas, carteles,
  estandartes, cajas de shulker, cabezas, estanterías…): un cambio de
  familia o un «parecido» no debe crear ni sustituir contenedores, y
  `aplicar_cambios` quita la entidad de bloque donde cambia el tipo, así que
  proponerlos invitaría a **perder su contenido**.

Sí pueden ser el **origen** de una búsqueda (pinchar un cofre en el visor
muestra bloques de su forma), pero nunca aparecen como destino.

### Familias de material

La familia es la raíz común de un bloque tras quitarle la forma y el
acabado; la **variante**, el acabado quitado. Sirve para cambiar un material
entero por otro conservando la correspondencia entre piezas.

1. Se quita el sufijo de forma (`_slab`, `_stairs`, `_wall`, `_fence`…).
   Las alfombras de color van con la lana de su color (`white_carpet` →
   `white_wool`).
2. **Maderas**: tablones, troncos, madera y sus descortezados se agrupan por
   especie (`oak`, `dark_oak`, `crimson`, `bamboo`…). Las formas de madera
   (`oak_stairs`, `oak_door`…) son de la variante `tablones`; `oak_log` y
   `crimson_stem`, `tronco`; `oak_wood` y `crimson_hyphae`, `madera`;
   `stripped_` añade `descortezado`; `bamboo_mosaic` es `mosaico`. Hojas,
   brotes y carteles no son madera: cada uno es su propia familia.
3. **Resto**: se quitan, estén donde estén, `cobbled` (adoquín), `polished`
   (pulido), `chiseled` (cincelado), `cracked` (agrietado), `smooth` (liso),
   `cut` (cortado), `mossy` (musgoso), `waxed` (encerado), `gilded`
   (dorado), `bricks`/`brick` (ladrillo), `tiles`/`tile` (baldosa), `pillar`
   (pilar), `grate` (rejilla), `bulb` (bombilla) y `block` (sin variante).
   Lo que queda es la familia: `cracked_polished_blackstone_bricks` →
   `blackstone`, variante `agrietado+pulido+ladrillo`;
   `waxed_exposed_cut_copper_slab` → `exposed_copper`, `encerado+cortado`.
4. **Excepciones**:
   - `bricks` (ladrillo de arcilla) es la familia `bricks`, variante
     `ladrillo` (si no, se quedaría sin raíz).
   - `nether_bricks` y `red_nether_bricks` son familias propias (no
     `nether`).
   - `cobblestone` es la variante `adoquín` de `stone` (como
     `cobbled_deepslate` de `deepslate`); `mossy_cobblestone`,
     `musgoso+adoquín`.
   - `dark_prismarine` es la variante `oscuro` de `prismarine`.
   - Las placas con peso son de `iron` (`heavy_`) y de `gold` (`light_`).
   - El bambú planta es `bamboo_plant` (no la madera `bamboo`).
   - Salen solas de las reglas: `quartz` (`quartz_block`, `quartz_bricks`,
     `quartz_pillar`, `smooth_quartz`, `chiseled_quartz_block`),
     `sandstone`/`red_sandstone`, `prismarine`, `end_stone`
     (`end_stone_bricks`), `purpur`, `tuff`, `mud` (`mud_bricks`) y el cobre
     por grado de oxidación (`copper`, `exposed_copper`…). Las menas
     (`deepslate_coal_ore`, `nether_quartz_ore`) son familias propias.
   - Ids de otros namespaces: mismas reglas, con el namespace delante
     (`mod:granito`).

**Equivalencia entre variantes.** Para emparejar piezas de dos familias se
usa un coste entre variantes: la suma de los pesos de los componentes que no
comparten (ladrillo y baldosa 1,5; musgoso 0,75; adoquín y agrietado 0,5; el
resto 1), con los pares afines ladrillo/baldosa y pulido/liso a 0,25, menos
la mitad del peso de lo que comparten. Así:

- `deepslate_brick_stairs` (ladrillo) → `polished_blackstone_brick_stairs`
  (pulido+ladrillo), no `blackstone_stairs`;
- `cobbled_deepslate_stairs` (adoquín) → `blackstone_stairs` (la piedra negra
  no tiene adoquín: su base es la equivalente);
- `polished_deepslate_slab` → `polished_blackstone_slab` (exacta);
- `deepslate_tile_wall` (baldosa) → `polished_blackstone_brick_wall`;
- y al revés, `polished_blackstone_bricks` → `deepslate_bricks`.

### Comandos

Todos trabajan en segundo plano. Los block states se validan como en
`aplicar_cambios`; los nombres de familia, con minúsculas, cifras y `_-.:/`
(máximo 128 caracteres): si no, `edicionInvalida`. `n` es opcional (12 por
defecto) y se acota a 1–100.

#### `parecidos(estado, n?) -> Parecido[]`

Los `n` bloques sugeribles de la misma forma más cercanos en color al
estado (sin el propio bloque), del más al menos parecido. Cada uno trae
`id`, `estado` (el destino ya construido), `forma`, `familia`, `color`
(`[r, g, b]` del estado destino) y `distancia` (OKLab). Vacío si el bloque
no está en el catálogo (mods) o no tiene color.

#### `familia_de(estado) -> FamiliaBloque`

`{ familia, variante, nombreVariante, forma, sugerible }`; `variante` es la
lista de componentes (`["pulido", "ladrillo"]`; vacía = base) y
`nombreVariante`, `base` o `pulido+ladrillo`. Funciona con cualquier id,
esté o no en el catálogo.

#### `miembros_familia(id, familia) -> EstadoPresente[]`

Los block states de esa familia presentes en el schematic (todas las
regiones, sin aire), con cuántas posiciones ocupa cada uno
(`{ estado, cantidad }`), ordenados por estado. Para seleccionar el grupo al
pinchar un bloque: `familiaDe(bloque.estado)` y después
`miembrosFamilia(id, familia)`.

#### `familias_parecidas(familia, n?) -> FamiliaParecida[]`

Familias ordenadas por `puntuacion` (menor = más parecida):

```
puntuacion = distancia OKLab entre los colores medios
           + 0,20 × (fracción de formas de la origen que le faltan)
           + 0,05 × (1 − variantesComunes)
```

El color medio es la media en OKLab de los miembros sugeribles (en la
origen, de todos si ninguno lo es). `variantesComunes` (0–1) es la media,
para cada variante de la origen, del parecido con la más equivalente de la
destino. Se descartan las familias sin ningún miembro sugerible con color y
las que no comparten ninguna forma con la origen. Cada una trae `familia`,
`color`, `distancia`, `formasComunes`, `variantesComunes`, `puntuacion` y sus
`miembros` sugeribles (`{ id, forma, variante, sugerible }`). Vacío si la
familia no existe. Con la tabla integrada, `deepslate` da primero
`blackstone`.

#### `plan_familia(id, familiaOrigen, familiaDestino) -> Reemplazo[]`

Un reemplazo por cada block state de la familia origen presente en el
schematic, ordenados por estado origen. El destino de cada uno:

1. Entre los miembros sugeribles de la familia destino **de la misma
   forma**, el de variante más equivalente (coste mínimo; la exacta si
   existe); a igual coste, el más cercano en color; a igual color, por id.
2. Si la familia destino no tiene esa forma (el basalto no tiene
   escaleras), el bloque sugerible de esa forma más parecido en color de
   **todo el catálogo** (el primero de `parecidos`).
3. Si tampoco hay (un `otro` sin equivalentes), ese estado no entra en el
   plan y se queda como está.

Los estados que no cambiarían tampoco entran (origen = destino da un plan
vacío). Error `edicionInvalida` si la familia destino no existe. El plan se
aplica tal cual: `aplicarCambios(id, [], plan)`, y queda en el historial
como cualquier otro cambio.
