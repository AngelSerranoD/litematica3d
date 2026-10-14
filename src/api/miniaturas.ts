// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Miniaturas isométricas de la biblioteca: comandos de Rust y URL del PNG
// guardado (docs/miniaturas.md). Sin Three.js: el render va en su propio trozo.

import { convertFileSrc } from "@tauri-apps/api/core";
import type { Entrada, Miniatura } from "./biblioteca";
import { llamar } from "./llamar";
import { pedirMalla, type Malla } from "./visor";

/**
 * Versión del render de las miniaturas. Subirla (otro encuadre, otra luz…)
 * deja todas las guardadas obsoletas: se regeneran solas.
 */
const VERSION_RENDER = 1;

/** Cabeceras de `guardar_miniatura` (las mismas que lee `comandos/visor.rs`). */
export const CABECERA_ID = "miniatura-id";
export const CABECERA_TEXTURAS = "miniatura-texturas";

/** Lo que admite Rust en la clave: ASCII visible y como mucho 100 caracteres. */
const MAX_CLAVE = 100;

/**
 * Clave de texturas de una miniatura: versión del render y versión de
 * Minecraft del atlas con que se dibujó, o «-» si fue con colores planos.
 * Ej.: `r1|1.21.4`, `r1|-`.
 */
export function claveTexturas(versionAtlas: string | null): string {
  const version = versionAtlas ? versionAtlas.replace(/[^A-Za-z0-9._+-]/g, "_") : "-";
  return `r${VERSION_RENDER}|${version}`.slice(0, MAX_CLAVE);
}

/**
 * Malla del schematic entero para su miniatura: Rust espera a que el atlas
 * esté preparado (o sale con colores si no hay jar) y no toca la caché del
 * visor.
 */
export const mallaMiniatura = (id: string): Promise<Malla> =>
  pedirMalla("malla_miniatura", { id });

/**
 * Guarda el PNG de la miniatura. Va como cuerpo binario de la petición (sin
 * JSON); el id y la clave, en cabeceras. Rust decide el nombre del archivo a
 * partir del id. Devuelve la entrada con su `miniatura` al día.
 */
export async function guardarMiniatura(
  id: string,
  texturas: string,
  png: Uint8Array,
): Promise<Entrada> {
  return llamar<Entrada>("guardar_miniatura", png, {
    headers: { [CABECERA_ID]: id, [CABECERA_TEXTURAS]: texturas },
  });
}

/**
 * URL del PNG por el protocolo `asset` (su alcance está limitado a
 * `<datos>/biblioteca/*.miniatura.png` en tauri.conf.json). La fecha va en la
 * consulta para que el webview no enseñe la versión anterior al regenerarla.
 */
export const urlMiniatura = (m: Miniatura): string =>
  `${convertFileSrc(m.ruta)}?v=${m.fecha}`;
