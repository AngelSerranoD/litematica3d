// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import type { AtlasCargado } from "../api/atlas";
import { nombresItems } from "../api/materiales";
import type { CaraMaterial, Malla } from "../api/visor";

/**
 * Lo que la pantalla de materiales necesita y no calcula ella:
 *
 * - los nombres de los ítems en inglés (una vez por sesión; si llegan
 *   vacíos, sin Minecraft, se vuelven a pedir al abrir otra vez el panel);
 * - del modelo que tiene abierto el visor, la Y de su origen (para pasar las
 *   capas del deslizador a coordenadas del schematic) y un icono por bloque
 *   sacado del atlas de texturas. El visor lo publica al cargar la malla.
 */

/**
 * Icono de un ítem: su color plano y, si hay atlas, la esquina superior
 * izquierda de su textura útil en el PNG del atlas (píxeles).
 */
export interface IconoItem {
  color: string;
  textura: { x: number; y: number } | null;
}

/** El PNG del atlas, listo para usarlo como fondo CSS. */
export interface AtlasIconos {
  url: string;
  ancho: number;
  alto: number;
  /** Lado de la textura útil (16): el icono se escala a partir de él. */
  tamTextura: number;
}

export interface ModeloAbierto {
  /** Schematic al que pertenece (para no mezclar con el anterior). */
  id: string;
  /** Y de la capa 0 del visor en coordenadas del schematic. */
  origenY: number;
  /** Por ID de bloque sin propiedades (`minecraft:oak_log`). */
  iconos: Map<string, IconoItem>;
  /** `null` sin texturas, o mientras se prepara la imagen. */
  atlas: AtlasIconos | null;
}

interface EstadoMateriales {
  /** `null` mientras no han llegado; `{}` si no hay Minecraft. */
  nombres: Record<string, string> | null;
  modelo: ModeloAbierto | null;
  cargarNombres: () => void;
  publicarModelo: (id: string, malla: Malla, atlas: AtlasCargado | null) => void;
}

/** Caras preferidas para el icono: el lateral es lo que identifica casi todo. */
const PREFERENCIA_CARA: CaraMaterial[] = [
  "sur",
  "norte",
  "este",
  "oeste",
  "cruz",
  "arriba",
  "abajo",
];

function iconosDe(malla: Malla, conTexturas: boolean): Map<string, IconoItem> {
  const mejores = new Map<string, { rango: number; icono: IconoItem }>();
  for (const m of malla.materiales) {
    const conTextura = conTexturas && m.textura !== null;
    // Una textura siempre gana a un color; entre iguales, la cara preferida.
    const rango = (conTextura ? 0 : 10) + PREFERENCIA_CARA.indexOf(m.cara);
    const actual = mejores.get(m.nombre);
    if (actual && actual.rango <= rango) continue;
    const a = malla.atlas;
    const indice = m.textura;
    const icono: IconoItem = {
      color: `rgb(${m.color[0]} ${m.color[1]} ${m.color[2]})`,
      textura:
        conTextura && a && indice !== null
          ? {
              x: (indice % a.celdasPorFila) * a.tamCelda + a.relleno,
              y: Math.floor(indice / a.celdasPorFila) * a.tamCelda + a.relleno,
            }
          : null,
    };
    mejores.set(m.nombre, { rango, icono });
  }
  return new Map([...mejores].map(([nombre, { icono }]) => [nombre, icono]));
}

/** URL del PNG del atlas por imagen: se genera una vez y se libera al cambiar. */
let urlAtlas: { imagen: ImageBitmap; url: Promise<string | null> } | null = null;

/**
 * El atlas en memoria viene volteado (v = 0 abajo, para la GPU): se vuelve a
 * voltear en un lienzo y se exporta a PNG, así las filas cuentan desde arriba
 * como en las cuentas de la celda.
 */
function urlDeAtlas(imagen: ImageBitmap): Promise<string | null> {
  if (urlAtlas?.imagen === imagen) return urlAtlas.url;
  const anterior = urlAtlas;
  const url = new Promise<string | null>((resolver) => {
    try {
      const lienzo = document.createElement("canvas");
      lienzo.width = imagen.width;
      lienzo.height = imagen.height;
      const ctx = lienzo.getContext("2d");
      if (!ctx) return resolver(null);
      ctx.translate(0, imagen.height);
      ctx.scale(1, -1);
      ctx.drawImage(imagen, 0, 0);
      lienzo.toBlob((blob) => resolver(blob ? URL.createObjectURL(blob) : null), "image/png");
    } catch {
      // Imagen ya cerrada (se invalidó el atlas): sin texturas.
      resolver(null);
    }
  });
  urlAtlas = { imagen, url };
  anterior?.url.then((u) => u && URL.revokeObjectURL(u));
  return url;
}

let pidiendoNombres = false;

export const useMateriales = create<EstadoMateriales>()((set, get) => ({
  nombres: null,
  modelo: null,

  cargarNombres: () => {
    // Vacío (sin Minecraft todavía) se vuelve a pedir la próxima vez.
    const nombres = get().nombres;
    if ((nombres !== null && Object.keys(nombres).length > 0) || pidiendoNombres) return;
    pidiendoNombres = true;
    nombresItems()
      .then((nombres) => set({ nombres }))
      .catch((e) => {
        // Sin nombres no se rompe nada: se humanizan los IDs.
        console.warn("No se pudieron cargar los nombres de los ítems:", e);
        set({ nombres: {} });
      })
      .finally(() => {
        pidiendoNombres = false;
      });
  },

  publicarModelo: (id, malla, atlas) => {
    const conTexturas = !!atlas && !!malla.atlas && atlas.info.version === malla.atlas.version;
    const modelo: ModeloAbierto = {
      id,
      origenY: malla.origen[1],
      iconos: iconosDe(malla, conTexturas),
      atlas: null,
    };
    set({ modelo });
    if (!conTexturas || !atlas || !malla.atlas) return;
    const datos = malla.atlas;
    void urlDeAtlas(atlas.imagen).then((url) => {
      // Si mientras tanto se publicó otro modelo, este ya no manda.
      if (!url || get().modelo !== modelo) return;
      set({
        modelo: {
          ...modelo,
          atlas: { url, ancho: datos.ancho, alto: datos.alto, tamTextura: datos.tamTextura },
        },
      });
    });
  },
}));
