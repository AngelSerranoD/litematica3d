# Visor 3D: contrato entre el núcleo Rust y el frontend

<!-- Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Este documento fija cómo llega la geometría del schematic al frontend. El mallado
lo hace Rust (`src-tauri/src/malla/`) y el frontend solo crea buffers de GPU con
lo que recibe. Decodificador de referencia: `src/api/visor.ts`. En Rust hay un
lector espejo (`malla::binario::leer`) que usan las pruebas.

## 1. Conceptos

- **Coordenadas del modelo**: bloques relativos a la esquina mínima de la caja
  que engloba todas las regiones. Siempre ≥ 0. Son las de los vértices, las del
  picking y las del corte por capas. Para pasar a coordenadas del schematic se
  suma `origen` (cabecera).
- **Regiones combinadas**: cada región se coloca en su `origen`. Si dos se
  solapan, gana el bloque no aire de la que va después.
- **Chunk**: cubo de 16³ bloques alineado a múltiplos de 16 en coordenadas del
  modelo. Solo se envían los chunks con geometría.
- **1/16 de bloque**: unidad de posiciones y UV (la de los modelos de
  Minecraft). Una caja de losa va de 0 a 8 en Y.
- **Material**: pareja (estado visual, cara). Cada vértice lleva el índice de su
  material en la tabla `materiales`.

## 2. Comandos

| Comando     | Argumentos                                   | Devuelve                              |
|-------------|----------------------------------------------|---------------------------------------|
| `malla`     | `{ id: string, capas: [number, number] \| null }` | `ArrayBuffer` con el binario (sección 3) |
| `malla_miniatura` | `{ id: string }` | El mismo binario, del modelo entero; espera al atlas y no usa la caché (docs/miniaturas.md) |
| `bloque_en` | `{ id: string, x: number, y: number, z: number }` | `BloqueSeleccionado \| null`          |
| `versiones_minecraft` | — | `VersionMinecraft[]` (sección 11.1) |
| `ajustes`   | — | `Ajustes` |
| `guardar_ajustes` | `{ ajustes: Ajustes }` | `boolean`: `true` si cambian las texturas (sección 11.6) |
| `carpeta_minecraft_por_defecto` | — | `string \| null`: la `.minecraft` habitual que existe |
| `atlas`     | — | `ArrayBuffer` con el PNG del atlas; **vacío** (`byteLength === 0`) si no hay ningún jar |
| `atlas_info` | — | `DatosAtlas \| null` (el atlas ya preparado; no genera nada) |

Funciones TS ya hechas en `src/api/visor.ts`:

```ts
cargarMalla(id: string, capas?: [number, number]): Promise<Malla>
bloqueEn(id: string, x: number, y: number, z: number): Promise<BloqueSeleccionado | null>
decodificarMalla(buffer: ArrayBuffer): Malla
celdaImpactada(punto: Vec3, normal: Vec3): Vec3
chunkEnCapas(c: ChunkMalla, desde: number, hasta: number): boolean
NORMALES: readonly Vec3[]

interface BloqueSeleccionado {
  estado: string;                       // "minecraft:oak_stairs[facing=north,half=bottom]"
  nombre: string;                       // "minecraft:oak_stairs"
  propiedades: Record<string, string>;  // { facing: "north", half: "bottom" }
  posicion: [number, number, number];   // coordenadas del schematic
  region: string | null;
}
```

Los errores llegan como `ErrorApp` (`{ tipo, mensaje }`), igual que en el resto
de comandos.

**Caché**: Rust guarda en memoria el último schematic abierto ya combinado
(`CacheVisor`). `malla` lo crea si no está; `bloque_en` y las llamadas a `malla`
con otras `capas` lo reutilizan sin releer el archivo. Borrar el schematic de la
biblioteca lo saca de la caché.

## 3. Formato binario (versión 1)

Todo **little endian**. Cada sección empieza en un múltiplo de 4 bytes contando
desde el inicio del `ArrayBuffer`, así que se pueden crear `Uint16Array` y
`Uint32Array` sobre el mismo buffer sin copiar (los typed arrays usan el orden
de bytes de la máquina, que es little endian en todas las plataformas de
escritorio soportadas).

### 3.1 Cabecera general (48 bytes)

| Offset | Tipo     | Campo          | Significado                                          |
|-------:|----------|----------------|------------------------------------------------------|
| 0      | 4 × u8   | magia          | `"L3DM"`                                             |
| 4      | u32      | version        | `1`. Cambia solo si se rompe la compatibilidad       |
| 8      | u32      | tamCabecera    | `48`. El JSON empieza aquí                           |
| 12     | u32      | nChunks        | Chunks que siguen                                    |
| 16     | 3 × i32  | origen         | Esquina mínima del modelo en coordenadas del schematic |
| 28     | 3 × u32  | tamano         | Ancho (X), alto (Y), largo (Z) en bloques            |
| 40     | u32      | bytesJson      | Longitud del JSON en bytes (sin relleno)             |
| 44     | u32      | reservado      | `0`                                                  |
| 48     | UTF-8    | JSON           | Sección 4. Relleno con ceros hasta múltiplo de 4     |

### 3.2 Cada chunk (cabecera de 40 bytes + datos)

Offsets relativos al inicio del chunk:

| Offset | Tipo    | Campo          | Significado                                                    |
|-------:|---------|----------------|----------------------------------------------------------------|
| 0      | 3 × i32 | origen         | Esquina mínima del chunk en bloques (modelo), múltiplo de 16    |
| 12     | i32     | yMin           | Primera capa Y (modelo) con geometría                           |
| 16     | i32     | yMax           | Última capa Y (modelo) con geometría, inclusiva                 |
| 20     | u32     | nVertices      |                                                                |
| 24     | u32     | nIndices       | Múltiplo de 6 (dos triángulos por quad)                         |
| 28     | u32     | nIndicesOpacos | Los primeros `nIndicesOpacos` son opacos o de recorte; el resto, translúcidos |
| 32     | u32     | bytesIndice    | `2` (u16) si `nVertices ≤ 65 536`; si no, `4` (u32)              |
| 36     | u32     | bytesDatos     | Bytes de datos que siguen (con rellenos): el siguiente chunk empieza en `40 + bytesDatos` |

Datos, en este orden, cada uno rellenado con ceros hasta múltiplo de 4:

| Sección     | Tipo                     | Elementos       | Contenido |
|-------------|--------------------------|-----------------|-----------|
| posiciones  | u16                      | 3 × nVertices   | x, y, z relativos a `origen` del chunk, en 1/16 de bloque (0..256) |
| normales    | u8                       | nVertices       | Índice de normal (tabla 3.3) |
| uvs         | u16                      | 2 × nVertices   | u, v en 1/16 de bloque (sección 5) |
| materiales  | u16                      | nVertices       | Índice en `materiales` |
| indices     | u16 o u32 (`bytesIndice`) | nIndices        | Triángulos en sentido antihorario visto desde fuera (cara frontal de Three.js) |

Coste: 13 bytes por vértice más 1,5 índices por vértice (3 bytes con u16).
Atributos en arrays separados (no intercalados): cada uno es un
`BufferAttribute` directo de Three.js.

### 3.3 Índices de normal

| Índice | Cara   | Vector              |
|-------:|--------|---------------------|
| 0      | arriba | (0, 1, 0)           |
| 1      | abajo  | (0, −1, 0)          |
| 2      | norte  | (0, 0, −1)          |
| 3      | sur    | (0, 0, 1)           |
| 4      | oeste  | (−1, 0, 0)          |
| 5      | este   | (1, 0, 0)           |
| 6..9   | cruz   | (1,0,1), (−1,0,−1), (1,0,−1), (−1,0,1), normalizados |

Los índices 0..5 coinciden con la cara del material. En el shader basta una
tabla constante de 10 vectores (`NORMALES` en `visor.ts`).

## 4. JSON: materiales y paleta

```json
{
  "materiales": [
    { "estado": "minecraft:oak_log[axis=x]", "nombre": "minecraft:oak_log",
      "cara": "norte", "tipo": "opaco", "color": [109, 85, 51], "textura": 619,
      "rotacion": 270 }
  ],
  "paleta": ["minecraft:air", "minecraft:oak_log[axis=x]", "..."],
  "totalBloques": 1234,
  "atlas": { "version": "1.21.8", "ancho": 2048, "alto": 1024, "celdasPorFila": 64,
             "tamCelda": 32, "tamTextura": 16, "relleno": 8,
             "celdaUv": [0.015625, 0.03125], "nivelesMip": 3 }
}
```

**Compatibilidad**: `rotacion` y `atlas` son campos nuevos del JSON; el
binario no cambia y la versión sigue siendo `1`. El decodificador actual de
`src/api/visor.ts` sigue funcionando (pasa `materiales` tal cual y no mira
`atlas`); para usar las texturas hay que añadir `rotacion: number` a
`MaterialMalla`, `atlas: DatosAtlas | null` a `Malla` y copiar `json.atlas` en
`decodificarMalla`.

- `estado`: block state sin las propiedades que no cambian el aspecto
  (`distance`, `persistent`, `waterlogged`, y `level` del agua y la lava). Así
  las hojas con distinta `distance` comparten material y se fusionan.
- `cara`: `arriba`, `abajo`, `norte`, `sur`, `oeste`, `este` o `cruz` (planos
  cruzados de las plantas). Va por separado porque troncos, hornos, hierba… usan
  texturas distintas en cada cara.
- `tipo`: pasada de render. `opaco`; `recorte` (hojas, plantas, raíles,
  barrotes: alphaTest sin mezcla); `translucido` (cristal, agua, hielo, slime,
  miel, paneles de cristal: mezcla alfa, `depthWrite` desactivado, al final).
- `color`: RGB 0..255 de respaldo.
- `textura`: celda del atlas o `null` (sin atlas, o bloque sin textura: bloques
  de mods, IDs antiguos sin traducción…). Con textura, `color` es su color
  medio; sin ella, el de `colores_bloques.json` (sección 11.4).
- `rotacion` *(nuevo, aditivo)*: giro de la textura en la cara, en grados
  (`0`, `90`, `180`, `270`), en sentido horario mirando la cara desde fuera.
  Lo llevan, por ejemplo, los lados de los troncos tumbados (la veta va en
  horizontal). `0` cuando no hay textura.
- `atlas` *(nuevo, aditivo, en la raíz del JSON)*: `DatosAtlas` del atlas con
  el que se resolvieron las celdas, o `null` si la malla va solo con colores
  (sección 11.2).
- `paleta`: paleta común de todo el schematic (índice 0 = aire), para leyendas o
  filtros.

Máximo 65 536 materiales (el índice va en u16); si se superan, `malla` falla con
un error interno. En la práctica un schematic grande tiene unos cientos.

## 5. UV y texturas que se repiten en quads fusionados

El greedy meshing une caras coplanarias del mismo material en un solo quad de
hasta 16 × 16 bloques. Para que la textura se repita una vez por bloque:

- Las UV van **en unidades de bloque × 16** (no 0..1 por quad) y dependen solo
  de la posición del vértice. Un quad de 4 × 3 bloques tiene u de 0 a 64 y v de
  0 a 48 (más un desplazamiento entero que da igual).
- El **material va aparte**, en su propio atributo, no metido en la UV.
- El shader hace `local = fract(uv / 16.0)`, lo gira según `rotacion` y lo
  lleva a la celda del atlas del material (fórmulas en la sección 11.2). Para
  evitar costuras con mipmaps en los bordes de `fract`, muestrear con
  `textureGrad` usando las derivadas de `uv / 16.0` (sin `fract`), escaladas
  a la celda; el relleno de cada celda es la propia textura repetida, así que
  el filtrado en el borde empalma con el lado opuesto.
- Orientación: mirando la cara desde fuera, u crece hacia la derecha y v hacia
  arriba. En la cara de arriba, v crece hacia el norte; en la de abajo, hacia el
  sur. Como dependen de la posición, una losa toma la mitad inferior de la
  textura y una alfombra la franja de abajo, igual que en Minecraft (v = 0 es
  el borde inferior de la imagen, el convenio de Three.js con `flipY`).
- Planos cruzados: u va de 0 a 16 a lo largo de la diagonal y v es la altura.

## 6. Qué se dibuja y qué se oculta

### 6.1 Clasificación (`src-tauri/src/malla/formas.rs`)

| Forma | Casos |
|-------|-------|
| Nada | `air`, `cave_air`, `void_air`, `structure_void`, `light`, `barrier`, `moving_piston` |
| Cubo translúcido | `water`, `bubble_column`, `glass`, `*_stained_glass`, `tinted_glass`, `ice`, `frosted_ice`, `slime_block`, `honey_block` |
| Cubo de recorte | `*_leaves`, `spawner`, `trial_spawner`, `mangrove_roots`, `scaffolding` |
| Planos cruzados | flores, `*_sapling`, `*_tulip`, hierbas y helechos, setas y hongos, raíces del Nether, cultivos, caña, algas, enredaderas colgantes, corales, tallos de melón y calabaza, telaraña… |
| Losas | `*_slab`: `bottom` 0..8, `top` 8..16, `double` = cubo |
| Escaleras | `*_stairs`: losa según `half` + escalón en la mitad de `facing`. Se ignora `shape` (esquinas) |
| Vallas | `*_fence`: poste 6..10 + dos travesaños por cada lado `true` |
| Puertas de valla | `*_fence_gate`: una hoja según `facing`; se ignoran `open` e `in_wall` |
| Muros | `*_wall`: poste 4..12 si `up`, brazos `low` (alto 14) o `tall` (16) |
| Barrotes y paneles | `iron_bars`, `glass_pane`, `*_glass_pane`: poste 7..9 + brazos |
| Láminas | `*_carpet`, `*_pressure_plate`, `snow` (`layers` × 2; 8 capas = cubo), `pink_petals`, `wildflowers`, `leaf_litter`, `lily_pad`, `redstone_wire`, raíles |
| Pegados a la pared | antorchas (de suelo y de pared, sin inclinar), escaleras de mano, botones, palancas, carteles, carteles colgantes, puertas (siempre cerradas), trampillas (según `open` y `half`), `vine`/`glow_lichen`/`sculk_vein`/`resin_clump` por lados |
| Cajas sueltas | cofres, faroles, cadenas, varas, macetas, cabezas, camas, cactus, tierra de cultivo, caminos, mesa de encantamientos, tarta, sensor de luz, hogueras, cortapiedras, repetidores, comparadores |
| Cubo opaco | todo lo demás, incluidos los bloques de mods y los IDs antiguos sin traducción |

Decisión: las plantas van como **dos planos cruzados** (cuatro quads, ambos
lados) y no como caja pequeña: con texturas de recorte se verán como en el
juego, y sin textura siguen siendo legibles. Llevan normales diagonales (6..9).

### 6.2 Ocultación de caras

Cada bloque declara qué parte de cada una de sus seis caras tapa por completo
(una máscara de 16 × 16 en 1/16 de bloque):

- Cubo opaco: las seis caras enteras.
- Modelo opaco: lo que sus cajas pegan al borde (la losa inferior tapa su cara
  de abajo; la escalera, la de abajo y la de su espalda; dos losas inferiores
  juntas se tapan media cara cada una).
- Cubos y modelos translúcidos o de recorte: nada.

Una cara (o la cara de una caja que toca el borde del bloque) no se emite si el
vecino tapa por completo su rectángulo. Además, dos cubos no opacos con el
**mismo nombre** (cristal con cristal, agua con agua, hojas con hojas) no
emiten la cara que comparten. Dentro de un mismo modelo se quitan las caras que
otra caja tapa entera (el fondo del escalón, los extremos de los travesaños).

### 6.3 Greedy meshing

Solo para caras de cubos completos: por cada dirección y cada capa del chunk se
hace una máscara 16 × 16 con el material visible y se fusionan rectángulos del
mismo material. Los modelos aproximados no se fusionan. No cruza chunks.

## 7. Corte por capas Y

Dos opciones que se combinan bien:

1. **Rápida (mientras se arrastra el control)**: planos de recorte en el shader
   (`material.clippingPlanes` en coordenadas del modelo) y ocultar los chunks
   enteros que no tocan el rango con `chunkEnCapas(c, desde, hasta)`
   (`yMax >= desde && yMin <= hasta`). Limitación: el corte queda hueco, se ve
   el interior sin tapas porque las caras internas no existen.
2. **Cerrada (al soltar el control)**: `cargarMalla(id, [desde, hasta])`. Rust
   vuelve a mallar desde la caché tratando lo de fuera del rango como aire, así
   que aparecen las caras del corte. Malla 1 M de bloques en ~15-80 ms más la
   transferencia.

## 8. Selección de bloques (picking)

1. Raycast de Three.js contra los meshes de los chunks; el punto de impacto se
   pasa a coordenadas del modelo (quitando la transformación del grupo).
2. `celdaImpactada(punto, normal)` retrocede una milésima de bloque contra la
   normal y redondea hacia abajo. No sirve retroceder medio bloque: en una
   alfombra o una placa de presión (1/16 de alto) caería en la celda de abajo.
3. `bloqueEn(id, x, y, z)` devuelve estado, nombre, propiedades, posición en
   coordenadas del schematic y región, leyendo de la caché de Rust.

## 9. Montaje recomendado en Three.js

- Un `Mesh` por chunk con `position = origen` y `scale = 1/16`; así los vértices
  `u16` se usan tal cual (`BufferAttribute(posiciones, 3)` sin normalizar).
- Atributos propios: `normalIdx` (`Uint8Array`, 1), `uvBloque` (`Uint16Array`,
  2), `material` (`Uint16Array`, 1). En GLSL llegan como `float`; con
  `ShaderMaterial` se busca la normal en una tabla constante y el color/celda
  del material en una textura de datos de `materiales.length` texels.
- `geometry.setIndex(new BufferAttribute(indices, 1))` y dos grupos:
  `addGroup(0, indicesOpacos, 0)` y
  `addGroup(indicesOpacos, nIndices - indicesOpacos, 1)` con dos materiales
  (opaco/recorte y translúcido). La caja de culling hay que darla a mano
  (`boundingSphere` de un chunk de 16³) o calcularla tras escalar.

## 10. Rendimiento medido

`cargo test --release --test malla_rendimiento -- --ignored --nocapture`
(100 × 100 × 100 sintéticos; con `LITEMATICA3D_VOLCAR_MALLA=<carpeta>` deja los
binarios para probar el decodificador):

| Caso | Bloques | Vértices | Triángulos | Binario | Combinar | Mallar | Serializar |
|------|--------:|---------:|-----------:|--------:|---------:|-------:|-----------:|
| Mezcla aleatoria (peor caso: 10 % aire, cristal, hojas, escaleras, losas, vallas y flores sueltas) | 899 911 | 9,38 M | 4,69 M | 143 MiB | 6 ms | 76 ms | 106 ms |
| Estructura (capas macizas + muros huecos + 3 % detalles) | 635 500 | 535 k | 267 k | 8,2 MiB | 5 ms | 13 ms | 8 ms |
| Macizo de piedra | 1 000 000 | 1 176 | 588 | 0,02 MiB | 5 ms | 8 ms | < 1 ms |

El mallado va en paralelo por chunks (rayon). El caso aleatorio es
deliberadamente malo (casi ningún bloque toca a otro igual); un schematic real
se parece más a «estructura».

## 11. Texturas

Las texturas salen del jar de Minecraft instalado en la máquina del usuario
(`src-tauri/src/texturas/`). **No se redistribuyen**: el atlas se genera la
primera vez y se guarda en la caché de la app. En el repositorio solo están
los colores medios (`src-tauri/datos/colores_bloques.json`), que son datos
derivados.

### 11.1 Versiones y ajustes

```ts
interface VersionMinecraft { id: string; ruta: string; esRelease: boolean }
interface Ajustes {
  versionMinecraft: string | null;        // null = automática (la primera de la lista)
  rutaMinecraft?: string;                 // sin ella, las carpetas habituales
  tema: "claro" | "oscuro" | "sistema";   // solo frontend: data-tema en <html>
  incluirFluidos: boolean;                // opción de la lista de materiales
}
interface DatosAtlas {
  version: string;             // versión de Minecraft del atlas
  ancho: number; alto: number; // píxeles, potencias de dos
  celdasPorFila: number;
  tamCelda: number;            // 32: textura + relleno a cada lado
  tamTextura: number;          // 16
  relleno: number;             // 8
  celdaUv: [number, number];   // [tamCelda / ancho, tamCelda / alto]
  nivelesMip: number;          // 3 = log2(relleno)
}

// Firmas sugeridas para src/api:
versionesMinecraft(): Promise<VersionMinecraft[]>  // invoke("versiones_minecraft")
ajustes(): Promise<Ajustes>                         // invoke("ajustes")
guardarAjustes(ajustes: Ajustes): Promise<boolean>  // invoke("guardar_ajustes", { ajustes })
carpetaMinecraftPorDefecto(): Promise<string | null> // invoke("carpeta_minecraft_por_defecto")
atlas(): Promise<ArrayBuffer | null>                // invoke("atlas"); byteLength 0 → null
atlasInfo(): Promise<DatosAtlas | null>             // invoke("atlas_info")
```

- **Detección**: `<.minecraft>/versions/<v>/<v>.jar`, buscando en
  `%APPDATA%\.minecraft`, `~/.minecraft` y
  `~/Library/Application Support/minecraft` (o solo en `rutaMinecraft` si está
  puesta). Solo cuentan los jar que traen `assets/minecraft/blockstates/` (hay
  perfiles de Fabric/Forge/OptiFine que solo traen el JSON o un jar sin
  assets). `esRelease`: `type: "release"` en el JSON de la versión y sin
  `inheritsFrom`; las derivadas (OptiFine…) van detrás aunque se declaren
  release. Orden: releases primero y, dentro, de la más reciente a la más
  antigua (`releaseTime`).
- **Versión usada**: `versionMinecraft` de los ajustes si está instalada; si no
  (o es `null`), la primera de la lista.
- **Ajustes**: `ajustes.json` en `app_config_dir` (lo escribe Rust, a un
  temporal y renombrando). Todos los campos tienen valor por defecto: un
  archivo antiguo sin `tema` ni `incluirFluidos` se sigue leyendo.
  `rutaMinecraft` vacía cuenta como ausente; si no, tiene que ser absoluta,
  existir y tener `versions/`, o `guardar_ajustes` falla con un error
  `carpetaMinecraft` y no guarda nada.
- **Cambio de texturas**: `guardar_ajustes` compara la versión que se usa de
  verdad (id y jar) antes y después. Si cambia, descarta el atlas activo y los
  nombres en memoria, empieza a preparar el nuevo atlas en segundo plano y
  devuelve `true`. Cambiar el tema o los fluidos (o elegir a mano la misma
  versión que la automática) devuelve `false` y no toca nada.

### 11.2 Atlas

- **Qué lleva**: todas las texturas de `textures/block/` (1083 en 1.21.8), no
  solo las del schematic: se genera una vez por versión y sirve para todos.
  Orden alfabético; la celda `i` está en la columna `i % celdasPorFila` y la
  fila `floor(i / celdasPorFila)`, contando filas desde **arriba** del PNG.
- **Celdas de 32 px**: la textura de 16 × 16 en el centro y 8 px de relleno a
  cada lado con la misma textura repetida (como `RepeatWrapping`). Así el
  filtrado bilineal y los mipmaps hasta el nivel 3 no mezclan celdas vecinas.
  Por encima del nivel 3 sí habría sangrado: limitar el LOD a `nivelesMip`
  (con `textureGrad`/`textureLod` en el shader) o usar `NearestFilter` sin
  mipmaps. Recomendado: `magFilter = NearestFilter` (píxel nítido, como el
  juego) y `minFilter = NearestMipmapLinearFilter` con el LOD limitado.
- **Animadas** (más altas que anchas: agua, lava, fuego, portal…): primer
  fotograma. Las que no miden 16 × 16 se escalan por vecino más cercano.
- **Tintes de bioma** fijos, ya aplicados en el PNG (solo a los bloques que el
  juego tinta; un `tintindex` heredado no basta: el caldero de lava hereda la
  cara tintada del de agua y el juego no la tinta):

  | Tinte | RGB | Bloques |
  |-------|-----|---------|
  | Hierba de llanura | `#91BD59` | `grass_block` (arriba), hierba, helechos, caña de azúcar, `pink_petals`, `wildflowers`, `bush` |
  | Follaje de llanura | `#77AB2F` | hojas de roble, jungla, acacia y roble oscuro, enredaderas |
  | Abedul / abeto / mangle | `#80A755` / `#619961` / `#92C648` | sus hojas |
  | Agua | `#3F76E4` | `water`, `bubble_column`, caldero con agua |
  | Nenúfar | `#208030` | `lily_pad` |
  | Hojarasca | `#A08250` | `leaf_litter` |
  | Redstone | `#B80000` | `redstone_wire` (media potencia) |
  | Tallos | `#E0C71C` | tallos de melón y calabaza |

  El lado de la hierba usa `grass_block_side`, que ya trae el borde verde; la
  capa `grass_block_side_overlay` no se compone.
- **PNG RGBA de 2048 × 1024** en 1.21.8 (2,1 MiB; 8 MiB en GPU). Generación:
  ~0,3 s en release (2 s en debug) leyendo el jar de 28 MiB.
- **Caché**: `<app_cache_dir>/texturas/<versión>/atlas.png` + `atlas.json`
  (`esquema`, `version`, `bytesJar`, tamaños, mapa `textura → celda` y color
  medio de cada celda). Se regenera si falta, si cambia `ESQUEMA_ATLAS` (hoy
  `1`) o si cambia el tamaño del jar. Con caché válida solo se leen del jar los
  JSON de blockstates y modelos (para resolver las caras).

**Del material a la UV del atlas** (GLSL, con `flipY = true`, el valor por
defecto de Three.js al cargar una imagen, así que v = 0 es el borde de abajo
del PNG):

```glsl
vec2 local = fract(uvBloque / 16.0);           // 0..1, u a la derecha, v arriba
// Giro de la textura (horario mirando desde fuera):
if (rotacion == 90.0)       local = vec2(1.0 - local.y, local.x);
else if (rotacion == 180.0) local = vec2(1.0 - local.x, 1.0 - local.y);
else if (rotacion == 270.0) local = vec2(local.y, 1.0 - local.x);
float col = mod(celda, celdasPorFila);
float fila = floor(celda / celdasPorFila);
vec2 px = vec2(col * tamCelda + relleno + local.x * tamTextura,
               fila * tamCelda + relleno + (1.0 - local.y) * tamTextura);
vec2 atlasUv = vec2(px.x / ancho, 1.0 - px.y / alto);
```

### 11.3 Del block state a la textura (`texturas/modelos.rs`)

1. **Blockstate**: en `variants`, la variante que encaja con más propiedades
   del estado sin contradecir ninguna (las que el estado no trae no descartan;
   si ninguna encaja, la primera); si es una lista con pesos, la primera
   entrada. En `multipart`, todas las partes cuyo `when` se cumple (valores con
   `|`, `OR` y `AND`), en orden.
2. **Modelo**: herencia por `parent` (las texturas del hijo mandan; los
   `elements` son los del primero de la cadena que los tenga) y variables
   `#nombre` resueltas.
3. **Cara**: se deshace el giro `x`/`y` de la variante para saber qué cara del
   modelo queda en esa cara del mundo, y se toma la del **primer elemento** que
   la tenga. `cruz`: la primera cara del primer elemento. Si ningún elemento
   tiene la cara (agua, lava, cofres, carteles, camas: el juego los dibuja
   aparte), la textura `particle`.
4. **Giro**: el borde superior de la imagen (arriba en los lados, norte en la
   cara de arriba, sur en la de abajo), con el `rotation` propio de la cara, se
   lleva al mundo con el giro de la variante y se compara con el de la cara del
   mundo. Con `uvlock: true` no hay giro.

Aproximaciones: se ignoran las `uv` parciales de las caras (las UV salen de la
posición, sección 5), los tintes por bioma real y el color de la redstone según
su potencia.

### 11.4 Colores de respaldo

`color_de(nombre)` (en `malla/apariencia.rs`) lee
`src-tauri/datos/colores_bloques.json` (`include_str!`): color medio ponderado
por alfa de la textura de arriba y la del norte del estado por defecto, con el
tinte aplicado (1058 bloques de 1.21.8). Lo que no está (bloques de mods)
cae al hash estable `color_hash`. Para regenerarla con otra versión:

```
cargo test --release --lib -- --ignored generar_colores_bloques --nocapture
# LITEMATICA3D_MC_VERSION=1.21.11 elige la versión; por defecto, 1.21.11 si
# tiene jar y si no la release más reciente instalada.
```

### 11.5 Cuándo hay texturas

`malla` usa el atlas activo si ya está preparado y nunca espera: si no lo
está, sale con `atlas: null` (solo colores) y deja la preparación en marcha.
El atlas se empieza a preparar al arrancar la app y al guardar ajustes.
Flujo recomendado en el frontend:

1. `atlas()` (async; la primera vez con una versión tarda lo que tarde en
   generarse) → crear la textura de Three.js. Si viene vacío, no hay jar:
   todo con colores.
2. `cargarMalla(...)`: si `malla.atlas` es `null` pero hay atlas, volver a
   pedir la malla; si `malla.atlas.version` no es la del PNG cargado
   (`atlasInfo()` justo después de `atlas()`), volver a pedir el PNG.

Punto de enganche en Rust: `textura_de(fuente, estado, cara)` en
`malla/apariencia.rs` recibe la `FuenteTexturas` (el atlas activo,
`texturas::Activo`) que `comandos::visor::malla` pasa a `mallar_con_texturas`;
`mallar` a secas sigue sin texturas (pruebas y rendimiento).

### 11.6 «Texturas cambiadas» en el frontend

`src/estado/texturas.ts`. Cuando `guardar_ajustes` devuelve `true`, los
ajustes llaman a `texturasCambiadas()`, que:

1. tira el atlas en memoria (`invalidarAtlas()`, `src/api/atlas.ts`);
2. tira los nombres de los ítems (`useMateriales.olvidarNombres()`);
3. sube `useTexturas.generacion`.

Quien dependa de las texturas escucha esa generación:

- el visor (`Visor3D`) vuelve a pedir la malla y el atlas del mismo schematic
  sin reiniciar las capas ni la cámara (primero sale con colores y, cuando el
  atlas nuevo está listo, con texturas);
- la pantalla de materiales vuelve a pedir los nombres;
- fuera de React: `alCambiarTexturas((generacion) => …)` devuelve la función
  para dejar de escuchar; la usa la cola de miniaturas para regenerarlas
  todas (docs/miniaturas.md).

## 12. Mejoras futuras

- **Modelos JSON completos** de Minecraft (blockstates + models con elementos,
  rotaciones y UV propias) en lugar de las cajas aproximadas: escaleras con
  esquinas (`shape`), puertas abiertas, antorchas inclinadas, yunques, calderos,
  tolvas, campanas, atriles, etc. Las UV explícitas ya caben en el formato.
- Altura real del agua y la lava según `level`, y el agua de los bloques
  `waterlogged`.
- Tintes de bioma por material (hoy son fijos y van horneados en el atlas) y
  la capa `grass_block_side_overlay` compuesta sobre el lado de la hierba.
- Texturas de resource packs (hoy solo el jar vanilla; las de 32 px se
  reducen a 16).
- Si la memoria aprieta: derivar las UV en el shader a partir de posición y
  normal (−4 bytes por vértice) o empaquetar posición y normal en un `u32`
  (requiere atributos enteros de WebGL2).
- Fusionar también caras de modelos (losas, alfombras) con greedy meshing.
