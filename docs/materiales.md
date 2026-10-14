# Pantalla de materiales: contrato entre el núcleo Rust y el frontend

<!-- Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados. -->

Este documento fija los comandos Tauri que usa la pantalla de materiales: la
lista de ítems de un schematic, la comprobación de lo ya reunido, los nombres en
inglés y la exportación a CSV. El código está en `src-tauri/src/comandos/materiales.rs`,
`src-tauri/src/materiales/`, `src-tauri/src/biblioteca.rs` y
`src-tauri/src/texturas/nombres.rs`. Todo funciona sin red.

Convenciones (las mismas que el resto de comandos):

- Los argumentos se pasan con los nombres de Rust (`id`, `item`, `reunido`,
  `ruta`, `contenido`, `opciones`); los campos de los objetos JSON van en
  camelCase.
- Un error llega como `{ "tipo": "...", "mensaje": "texto en español" }`
  (ver `src-tauri/src/error.rs`). `mensaje` se puede enseñar tal cual.

## 1. `materiales(id, opciones) -> ListaMateriales`

```ts
opciones: {
  incluirFluidos?: boolean;        // agua y lava (solo fuentes) como cubos; por defecto false
                                   // (el panel lo toma de Ajustes: `incluirFluidos` de ajustes.json)
  region?: string | null;          // solo esa región; null = todas
  yMin?: number | null;            // Y mínima inclusiva, coordenadas del SCHEMATIC
  yMax?: number | null;            // Y máxima inclusiva, coordenadas del SCHEMATIC
  capas?: [number, number] | null; // [desde, hasta] inclusivo, coordenadas del MODELO (visor)
}
```

Todos los campos son opcionales (los que falten valen su defecto).

```ts
ListaMateriales: {
  materiales: Array<{
    item: string;            // "minecraft:stone"; "schematiclab:desconocido" agrupa IDs antiguos sin traducción
    cantidad: number;
    tamanoStack: number;     // 64, 16 o 1
    stacks: number;          // stacks × tamanoStack + sueltos = cantidad
    sueltos: number;
    shulkers: number;        // cajas llenas (27 stacks)
    stacksEnShulker: number; // lo que queda fuera de las cajas llenas
    sueltosEnShulker: number;
  }>;                        // de más a menos cantidad; a igualdad, por item
  totalItems: number;
  totalStacks: number;       // huecos de inventario
  totalShulkers: number;     // ceil(totalStacks / 27)
  desconocidos: number;
}
```

Errores: `noEncontrado`, `archivoPerdido`, `archivoCorrupto`,
`regionNoEncontrada`.

### Coordenadas Y: visor y materiales

Hay dos sistemas de coordenadas:

- **Schematic**: las del archivo; cada región tiene su `origen` (el de
  `obtener` → `regiones[].origen`), que puede ser negativo.
- **Modelo**: las del visor (`src/estado/visor.ts`, `desde`/`hasta`) y del
  parámetro `capas` de `malla`. La capa 0 es la más baja de la caja que engloba
  todas las regiones con volumen.

Conversión: `y_schematic = y_modelo + origenY`, con `origenY` = la mínima
`origen[1]` de las regiones con volumen (0 si no hay ninguna). Es el mismo valor
que `origen[1]` en la cabecera binaria de la malla (`docs/visor.md`).

Para no tener que convertir nada, **el corte del visor se pasa tal cual en
`opciones.capas`**: `capas: [capaMin(visor), capaMax(visor)]`. El backend suma
`origenY` por su cuenta. Si además llegan `yMin`/`yMax`, se cuenta la
intersección de los dos rangos. Con el filtro de `region` también se usa el
`origenY` de todo el schematic (igual que el visor).

## 2. `comprobacion(id) -> string[]`

Ítems marcados como reunidos en ese schematic (`["minecraft:oak_planks",
"minecraft:stone"]`), ordenados por id. Vacío si no hay ninguno.

Errores: `noEncontrado` si el schematic no está en la biblioteca.

## 3. `marcar(id, item, reunido) -> null`

`reunido: true` marca el ítem y `false` lo desmarca. Es idempotente: marcar dos
veces o desmarcar uno sin marcar no da error. Se guarda en SQLite al momento.

Errores: `noEncontrado` (schematic que no existe), `nombreInvalido` (`item`
vacío o de más de 200 caracteres).

Persistencia: tabla `comprobacion (schematic_id, item)` de la biblioteca
(esquema v2, `PRAGMA user_version = 2`). Una biblioteca v1 se migra sola al
abrirla. Al eliminar un schematic se borran sus marcas: hay `ON DELETE CASCADE`
con las claves ajenas activadas y, además, `eliminar` las borra a mano en la
misma transacción.

## 4. `nombres_items() -> Record<string, string>`

Id de ítem → nombre en inglés: `{ "minecraft:stone": "Stone", ... }`. La
interfaz sigue en español; solo los nombres de ítems van en inglés (así se
llaman en el juego en inglés). Nunca da error: sin Minecraft o sin idioma en el
jar devuelve `{}` y el frontend humaniza el id en Title Case («Oak Planks»).

De dónde salen: `assets/minecraft/lang/en_us.json` dentro del jar de la versión
de Minecraft elegida en los ajustes (la misma de las texturas); en las versiones
modernas está siempre ahí, sin tocar los assets del launcher. Al cambiar de
versión o de carpeta en los ajustes (`texturasCambiadas()`, docs/visor.md 11.6)
el frontend tira los nombres y los vuelve a pedir.

Del archivo de idioma se toman las claves `block.minecraft.<x>` e
`item.minecraft.<x>` (sin más puntos) como `minecraft:<x>`; si están las dos,
gana la de ítem (`item.minecraft.oak_sign` antes que `block.minecraft.oak_sign`).
Las versiones 1.12 y anteriores usan `.lang` con otras claves (`tile.<x>.name`)
que no coinciden con los ids modernos: no se leen y sale `{}`.

Caché: en memoria por versión y en disco en
`<app_cache_dir>/texturas/<versión>/nombres_en.json` (junto al atlas). Si sale
vacío no se guarda en disco, para volver a mirarlo en el próximo arranque.

## 5. `guardar_csv(nombre, contenido) -> bool`

Abre desde Rust el diálogo de guardar (con `nombre` como propuesta; si a la
ruta elegida le falta `.csv`, se le añade) y escribe `contenido` en UTF-8, tal
cual (sin BOM; si Excel lo necesita para los acentos, el frontend puede empezar
el texto con `﻿`). Devuelve `false` si el usuario cancela. El frontend no
manda la ruta, así que no puede escribir fuera de lo que elija el usuario; por
eso la ventana tampoco tiene `dialog:allow-save`.

Por seguridad solo acepta rutas absolutas que acaben en `.csv` o `.txt` (sin
distinguir mayúsculas). Si no, error `nombreInvalido` y no escribe nada. Si
falla el disco, error `io`.
