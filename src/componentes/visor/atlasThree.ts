// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Del atlas decodificado (src/api/atlas.ts) a la textura de Three.js y a la
// tabla de celdas del shader (docs/visor.md 11.2).

import {
  LinearMipmapLinearFilter,
  NearestFilter,
  SRGBColorSpace,
  Texture,
} from "three";
import type { AtlasCargado } from "../../api/atlas";
import type { DatosAtlas } from "../../api/visor";
import type { AtlasVisor } from "./sombreado";

/** Rectángulo UV [u, v, ancho, alto] (v = 0 abajo) de una celda: solo la textura útil. */
function celdaDeAtlas(d: DatosAtlas): AtlasVisor["celda"] {
  const w = d.tamTextura / d.ancho;
  const h = d.tamTextura / d.alto;
  return (indice) => {
    const col = indice % d.celdasPorFila;
    const fila = Math.floor(indice / d.celdasPorFila);
    const u = (col * d.tamCelda + d.relleno) / d.ancho;
    // Las filas se cuentan desde arriba del PNG; con flipY, v = 0 es abajo.
    const v = 1 - (fila * d.tamCelda + d.relleno + d.tamTextura) / d.alto;
    return [u, v, w, h];
  };
}

/** Un atlas de Three por imagen: la caché de api/atlas.ts decide cuándo hay otra. */
let actual: { imagen: ImageBitmap; atlas: AtlasVisor } | null = null;

/** Textura y tabla de celdas del atlas cargado (la misma mientras la imagen sea la misma). */
export function atlasVisor(a: AtlasCargado): AtlasVisor {
  if (actual?.imagen === a.imagen) return actual.atlas;
  actual?.atlas.textura.dispose();
  const atlas = crearAtlasVisor(a);
  actual = { imagen: a.imagen, atlas };
  return atlas;
}

/**
 * Textura nueva (sin caché) y tabla de celdas de un atlas. Quien la crea la
 * libera: el renderizador de miniaturas usa la suya, aparte de la del visor,
 * para poder soltarla sin dejar al visor sin texturas.
 */
export function crearAtlasVisor(a: AtlasCargado): AtlasVisor {
  const textura = new Texture(a.imagen);
  // La imagen ya viene volteada (imageOrientation: "flipY"); Three ignora flipY con ImageBitmap.
  textura.flipY = false;
  textura.colorSpace = SRGBColorSpace;
  // Sin wrap: el relleno de 8 px de cada celda ya resuelve los bordes.
  textura.magFilter = NearestFilter; // píxel nítido, como el juego
  textura.minFilter = LinearMipmapLinearFilter;
  textura.generateMipmaps = true;
  textura.anisotropy = 4;
  textura.needsUpdate = true;
  return {
    textura,
    celda: celdaDeAtlas(a.info),
    tamano: [a.info.ancho, a.info.alto],
    nivelesMip: a.info.nivelesMip,
  };
}

