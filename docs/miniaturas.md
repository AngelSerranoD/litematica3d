# Miniaturas isométricas de la biblioteca

<!-- Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Cada schematic de la biblioteca tiene una miniatura: un render isométrico de su
contenido, con las mismas texturas que el visor 3D, que se genera en segundo
plano al importarlo y se guarda junto a su copia. Mientras no existe (o si no
se puede leer) se ve el marcador plano de siempre: el prisma de colores con
las proporciones del schematic. Todo funciona sin red.

## 1. Dónde se guarda

- PNG: `<datos>/biblioteca/<id>.miniatura.png`, junto a la copia
  `<id>.<ext>` (en Windows, `%APPDATA%\com.aserrano.litematica3d\biblioteca\`).
  La extensión de las copias nunca lleva puntos, así que los nombres no
  chocan.
- Índice SQLite, esquema v3 (`src-tauri/src/biblioteca.rs`): dos columnas
  nuevas en `schematics`, añadidas con `ALTER TABLE` al abrir una biblioteca
  v1 o v2 (las filas antiguas quedan sin miniatura):
  - `miniatura_fecha INTEGER`: cuándo se escribió el PNG (ms). `NULL` = no hay.
  - `miniatura_texturas TEXT`: clave de las texturas con que se generó.
    `NULL` = obsoleta.
- Cada `Entrada` lleva `miniatura: { ruta, fecha, texturas } | null`; `ruta`
  es absoluta y la calcula Rust a partir del id.

## 2. Clave de texturas

`r<VERSION_RENDER>|<versión de Minecraft del atlas>`, o `r<n>|-` si se
dibujó con colores planos (sin jar). Ejemplos: `r1|1.21.4`, `r1|-`.
`VERSION_RENDER` está en `src/api/miniaturas.ts`: subirla (otro encuadre,
otra luz…) deja todas obsoletas y se regeneran solas.

Una miniatura hay que (re)generarla si no existe, si su clave es `NULL` o si
no coincide con la clave de las texturas activas
(`necesitaMiniatura`, `src/estado/miniaturas.ts`). Si el atlas falla y no se
puede saber la clave activa, solo se hacen las que faltan o son `NULL`.

## 3. Comandos

### `malla_miniatura(id) -> ArrayBuffer`

El mismo binario que `malla` (docs/visor.md, sección 3), del modelo entero.
Diferencias: espera a que el atlas de la versión elegida esté preparado (si
falla o no hay jar, sale con colores) y no toca la caché del visor, para que
generar miniaturas no eche de ella el schematic que se está viendo.

### `guardar_miniatura` (cuerpo binario) -> Entrada

El PNG va como cuerpo de la petición, sin JSON:

```ts
invoke("guardar_miniatura", png /* Uint8Array */, {
  headers: { "miniatura-id": id, "miniatura-texturas": clave },
});
```

Rust comprueba que el id sea de la biblioteca (`noEncontrado` si no), que la
clave tenga de 1 a 100 caracteres ASCII visibles y que el PNG pese como mucho
4 MiB, empiece por la firma PNG, sea cuadrado de 64 a 1024 px y se decodifique
entero (si no, `miniaturaInvalida`). El nombre del archivo sale del id: el
frontend nunca manda rutas. Escribe en `<id>.miniatura.png.tmp` y renombra
encima de la anterior (con unos reintentos: en Windows el webview puede tener
abierta la vieja un instante). Devuelve la entrada con su `miniatura` al día.

### `guardar_ajustes`

Cuando cambian las texturas (devuelve `true`) también ejecuta
`UPDATE schematics SET miniatura_texturas = NULL`: quedan marcadas como
obsoletas en disco al momento, así que si se cierra la app antes de
regenerarlas, al volver a abrirla se sabe cuáles faltan. Los PNG viejos se
siguen enseñando hasta que llegan los nuevos.

### `eliminar`

Manda a la Papelera la copia, la miniatura y la carpeta de historial en una
sola operación (lo que ya no exista se salta) y quita sus filas.

### `aplicar_cambios` y `restaurar_version`

Al editar un schematic (docs/edicion.md) su miniatura queda obsoleta
(`miniatura_texturas = NULL` para ese id): el PNG viejo se sigue viendo hasta
que la cola lo regenera.

## 4. Render (`src/componentes/visor/miniatura3d.ts`)

- Un único `WebGLRenderer` con un lienzo de 512 × 512 que nunca entra en el
  DOM (`alpha`, fondo transparente, `preserveDrawingBuffer`, `autoClear`
  apagado). Se crea con la primera miniatura y se suelta
  (`dispose` + `forceContextLoss`) a los 10 s de vaciarse la cola.
- Misma malla y mismos materiales que el visor: `crearGrupo` (mallaThree.ts)
  y `crearMateriales` (sombreado.ts). La textura del atlas es propia
  (`crearAtlasVisor`), no la del visor, para poder soltarla sin dejarlo sin
  texturas.
- Cámara ortográfica isométrica desde +X +Y +Z (la vista del marcador plano):
  se proyectan las 8 esquinas de la caja del modelo y se encuadra un cuadrado
  centrado con un 4 % de aire.
- Por tandas de unos 300 000 vértices, cediendo el hilo entre ellas: primero
  los opacos y de recorte; después se liberan las geometrías sin nada
  translúcido y se dibujan los translúcidos del chunk más lejano al más
  cercano. Así un schematic enorme no congela la interfaz (6 millones de
  vértices: ~1,3 s repartidos en tandas). Entre tandas se pregunta si seguir:
  un cambio de texturas o un borrado abandonan el render.
- Al acabar, `toBlob` → PNG; geometrías y materiales se liberan siempre.

## 5. Cola (`src/estado/miniaturas.ts`)

App llama a `iniciarMiniaturas()` al arrancar. La cola:

1. espera a que la biblioteca termine de cargar y calcula la clave activa
   (`obtenerAtlas()`: la primera vez puede tardar lo que tarde en generarse
   el atlas);
2. mete en la cola, por orden de la lista, las entradas que necesitan
   miniatura, y vuelve a mirar cada vez que cambian las entradas (importar,
   deshacer un borrado…);
3. las genera de una en una, con 150 ms de respiro entre ellas:
   `malla_miniatura` → atlas de la misma versión que la malla → render →
   `guardar_miniatura` → actualiza solo el campo `miniatura` de la entrada en
   `useBiblioteca` (la tarjeta cambia sola);
4. con `alCambiarTexturas` (src/estado/texturas.ts): sube su generación
   (lo que estaba en marcha se descarta), marca en memoria todas como
   obsoletas, vuelve a calcular la clave y lo regenera todo.

Lo que falla (o un schematic sin nada visible) no se reintenta en la misma
sesión; un cambio de texturas vuelve a intentarlo todo.

## 6. Enseñarla

`Miniatura` (src/componentes/Miniatura.tsx) recibe `miniatura` de la entrada
(lámina, fila de la lista y marcador del visor). Con ella pinta un `<img>`
con `convertFileSrc(ruta)?v=<fecha>` (la fecha evita la caché del webview al
regenerarla); sin ella, o si la imagen no carga, el prisma plano. Las dos
llevan el mismo `layoutId`, así que la transición miniatura → visor sigue
igual.

El protocolo `asset` está activado (feature `protocol-asset` de `tauri` y
`app.security.assetProtocol` en tauri.conf.json) con el alcance limitado a
`$APPDATA/biblioteca/*.miniatura.png`: solo las miniaturas, ni las copias de
los schematics ni nada fuera de esa carpeta.

> Nota de desarrollo: si la app se lanza desde un proceso empaquetado (MSIX)
> que virtualiza `%APPDATA%`, los archivos nuevos acaban en
> `…\LocalCache\Roaming\…` y el alcance (que compara rutas canónicas) los
> rechaza con 403: se ve el marcador plano. Lanzada de forma normal no pasa.
