// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";
import { celdasAtlas, coloresBloques, MAX_CELDAS_ATLAS, type AtlasCargado, type CeldaAtlas } from "../api/atlas";
import { normalizarError } from "../api/llamar";
import { comprobacion, guardarCsv, marcar, nombresItems } from "../api/materiales";
import type { CaraMaterial, DatosAtlas, Malla } from "../api/visor";
import { csvLista, nombreArchivo, type Fila } from "../logica/materiales";
import { useBiblioteca } from "./biblioteca";

/**
 * Lo que la pantalla de materiales necesita y no calcula ella:
 *
 * - los nombres de los ítems en inglés (una vez por sesión; si llegan
 *   vacíos, sin Minecraft, se vuelven a pedir al abrir otra vez el panel);
 * - del modelo que tiene abierto el visor, la Y de su origen (para pasar las
 *   capas del deslizador a coordenadas del schematic) y un icono por bloque
 *   sacado del atlas de texturas. El visor lo publica al cargar la malla;
 * - la lista de comprobación (lo reunido) del schematic cuyo panel está
 *   abierto, con el marcado al momento, y la exportación a CSV.
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
  /** Datos completos del atlas (versión y rejilla de celdas), para iconos de otros bloques. */
  datos: DatosAtlas;
}

interface ModeloAbierto {
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
  /** Tira los nombres (cambiaron las texturas): se vuelven a pedir. */
  olvidarNombres: () => void;
  publicarModelo: (id: string, malla: Malla, atlas: AtlasCargado | null) => void;
  /**
   * Iconos de ítems que no salen de la malla (el ítem de una regla de
   * materiales, p. ej. `dirt` para un `dirt_path`), atados al modelo con el
   * que se resolvieron: si cambia el modelo o su atlas, ya no valen.
   */
  extras: { modelo: ModeloAbierto; iconos: Map<string, IconoItem> } | null;
  /** Resuelve por id de ítem el icono de los que falten en el modelo abierto. */
  completarIconos: (items: string[]) => void;
  /** Ítems reunidos del schematic `id` (`null` sin panel abierto o al cambiar de schematic). */
  comprobacion: { id: string; reunidos: ReadonlySet<string> } | null;
  /**
   * Empieza vacía y pide la lista de comprobación de `id`. Devuelve la
   * limpieza: la respuesta en vuelo se descarta y la lista se olvida.
   */
  cargarComprobacion: (id: string) => () => void;
  /** Marca o desmarca al momento; si el guardado falla, deshace ese ítem. */
  alternarReunido: (id: string, item: string, nombre: string, reunido: boolean) => void;
  /** Diálogo de guardado y CSV de las filas (con lo reunido), con su aviso. */
  exportarCsv: (titulo: string, filas: Fila[], reunidos: ReadonlySet<string>) => Promise<void>;
}

/** Orden de preferencia de caras en `CARAS_ATLAS`: sur, norte, este, oeste, cruz, arriba, abajo. */
const ORDEN_CARAS_ATLAS = [3, 2, 5, 4, 6, 0, 1];

/** Ítems ya pedidos para cada modelo (para no repetir llamadas). */
const pedidosExtras = new WeakMap<ModeloAbierto, Set<string>>();

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

/** La celda que mejor representa a un bloque (lateral antes que arriba/abajo). */
function celdaDeFila(fila: (CeldaAtlas | null)[] | undefined): CeldaAtlas | null {
  for (const i of ORDEN_CARAS_ATLAS) {
    const c = fila?.[i];
    if (c) return c;
  }
  return null;
}

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
/** Sube al olvidar los nombres: la petición en vuelo deja de valer. */
let generacionNombres = 0;
/** Sube con cada lista de comprobación pedida u olvidada. */
let peticionComprobacion = 0;

const avisar = (mensaje: string) => useBiblioteca.getState().avisar(mensaje);

export const useMateriales = create<EstadoMateriales>()((set, get) => ({
  nombres: null,
  modelo: null,
  extras: null,
  comprobacion: null,

  cargarNombres: () => {
    // Vacío (sin Minecraft todavía) se vuelve a pedir la próxima vez.
    const nombres = get().nombres;
    if ((nombres !== null && Object.keys(nombres).length > 0) || pidiendoNombres) return;
    pidiendoNombres = true;
    const mia = generacionNombres;
    nombresItems()
      .then((nombres) => {
        if (mia === generacionNombres) set({ nombres });
      })
      .catch((e) => {
        // Sin nombres no se rompe nada: se humanizan los IDs.
        console.warn("No se pudieron cargar los nombres de los ítems:", e);
        if (mia === generacionNombres) set({ nombres: {} });
      })
      .finally(() => {
        if (mia === generacionNombres) pidiendoNombres = false;
      });
  },

  olvidarNombres: () => {
    generacionNombres++;
    pidiendoNombres = false;
    set({ nombres: null });
  },

  publicarModelo: (id, malla, atlas) => {
    const conTexturas = !!atlas && !!malla.atlas && atlas.info.version === malla.atlas.version;
    const modelo: ModeloAbierto = {
      id,
      origenY: malla.origen[1],
      iconos: iconosDe(malla, conTexturas),
      atlas: null,
    };
    set({ modelo, extras: null });
    if (!conTexturas || !atlas || !malla.atlas) return;
    const datos = malla.atlas;
    void urlDeAtlas(atlas.imagen).then((url) => {
      // Si mientras tanto se publicó otro modelo, este ya no manda.
      if (!url || get().modelo !== modelo) return;
      set({
        extras: null, // el modelo cambia de objeto: los extras se vuelven a resolver
        modelo: {
          ...modelo,
          atlas: {
            url,
            ancho: datos.ancho,
            alto: datos.alto,
            tamTextura: datos.tamTextura,
            datos,
          },
        },
      });
    });
  },

  completarIconos: (items) => {
    const modelo = get().modelo;
    if (!modelo) return;
    let pedidos = pedidosExtras.get(modelo);
    if (!pedidos) pedidosExtras.set(modelo, (pedidos = new Set()));
    const faltan = [...new Set(items)].filter((i) => !modelo.iconos.has(i) && !pedidos.has(i));
    if (!faltan.length) return;
    faltan.forEach((i) => pedidos.add(i));
    // El ítem se busca como bloque del mismo id; si no lo es (un cubo, un huevo…),
    // no hay textura ni color y la fila se queda con el cuadrado neutro.
    const datos = modelo.atlas?.datos;
    const resolver = async (): Promise<Map<string, IconoItem>> => {
      const nuevos = new Map<string, IconoItem>();
      for (let i = 0; i < faltan.length; i += MAX_CELDAS_ATLAS) {
        const tanda = faltan.slice(i, i + MAX_CELDAS_ATLAS);
        const r = datos ? await celdasAtlas(tanda) : null;
        if (r && datos && r.atlas.version === datos.version) {
          tanda.forEach((item, k) => {
            const celda = celdaDeFila(r.celdas[k]);
            if (!celda) return;
            nuevos.set(item, {
              color: `rgb(${celda.color[0]} ${celda.color[1]} ${celda.color[2]})`,
              textura: {
                x: (celda.celda % datos.celdasPorFila) * datos.tamCelda + datos.relleno,
                y: Math.floor(celda.celda / datos.celdasPorFila) * datos.tamCelda + datos.relleno,
              },
            });
          });
        } else {
          // Sin atlas (sin jar): color plano.
          const colores = await coloresBloques(tanda);
          tanda.forEach((item, k) => {
            const c = colores[k];
            if (c) nuevos.set(item, { color: `rgb(${c[0]} ${c[1]} ${c[2]})`, textura: null });
          });
        }
      }
      return nuevos;
    };
    resolver()
      .then((nuevos) => {
        const s = get();
        // Si mientras tanto cambió el modelo (o su atlas), este resultado ya no vale.
        if (s.modelo !== modelo || !nuevos.size) return;
        const previos = s.extras?.modelo === modelo ? s.extras.iconos : new Map<string, IconoItem>();
        set({ extras: { modelo, iconos: new Map([...previos, ...nuevos]) } });
      })
      .catch((e) => console.warn("No se pudieron resolver los iconos de los ítems:", e));
  },

  cargarComprobacion: (id) => {
    const mia = ++peticionComprobacion;
    set({ comprobacion: { id, reunidos: new Set() } });
    comprobacion(id)
      .then((items) => {
        if (mia === peticionComprobacion) set({ comprobacion: { id, reunidos: new Set(items) } });
      })
      .catch((e) => {
        if (mia === peticionComprobacion)
          avisar(`No se pudo leer la lista de comprobación: ${normalizarError(e).mensaje}`);
      });
    return () => {
      if (mia !== peticionComprobacion) return;
      peticionComprobacion++;
      set({ comprobacion: null });
    };
  },

  alternarReunido: (id, item, nombre, reunido) => {
    const fijar = (valor: boolean) =>
      set((s) => {
        const c = s.comprobacion;
        if (c?.id !== id || c.reunidos.has(item) === valor) return {};
        const nuevos = new Set(c.reunidos);
        if (valor) nuevos.add(item);
        else nuevos.delete(item);
        return { comprobacion: { id, reunidos: nuevos } };
      });
    fijar(reunido);
    marcar(id, item, reunido).catch((e) => {
      fijar(!reunido);
      avisar(`No se pudo guardar «${nombre}»: ${normalizarError(e).mensaje}`);
    });
  },

  exportarCsv: async (titulo, filas, reunidos) => {
    try {
      const guardado = await guardarCsv(nombreArchivo(titulo), csvLista(filas, reunidos));
      if (!guardado) return;
      avisar("Lista de materiales guardada en CSV.");
    } catch (e) {
      avisar(`No se pudo guardar el CSV: ${normalizarError(e).mensaje}`);
    }
  },
}));
