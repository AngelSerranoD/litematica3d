// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Cola de miniaturas (docs/miniaturas.md): genera en segundo plano, de una en
// una, las que faltan o están obsoletas, y las regenera cuando cambian las
// texturas. Vive fuera de React; App la arranca con `iniciarMiniaturas()`.

import { obtenerAtlas, type AtlasCargado } from "../api/atlas";
import type { Entrada, ErrorApp } from "../api/biblioteca";
import { claveTexturas, guardarMiniatura, mallaMiniatura } from "../api/miniaturas";
import { dormir } from "../logica/tiempo";
import { useBiblioteca } from "./biblioteca";
import { alCambiarTexturas } from "./texturas";

// Three.js va en su propio trozo: solo se carga si hay miniaturas que hacer.
const cargarRender = () => import("../componentes/visor/miniatura3d");

/** Respiro entre miniatura y miniatura para la interfaz y el disco. */
const PAUSA_MS = 150;
/** Sin trabajo durante este tiempo, se suelta el contexto WebGL. */
const LIBERAR_TRAS_MS = 10_000;

/**
 * Clave de las texturas activas (`claveTexturas`). `undefined`: por calcular
 * (al arrancar o tras un cambio de texturas). `null`: no se pudo saber (el
 * atlas falló); entonces solo se hacen las que faltan o están obsoletas.
 */
let clave: string | null | undefined;
let calculando: Promise<void> | null = null;
/** Sube con cada cambio de texturas: el trabajo de antes ya no vale. */
let generacion = 0;
const cola: string[] = [];
const enCola = new Set<string>();
/** Fallaron (o no tienen nada que dibujar) en esta sesión: no se reintentan. */
const descartadas = new Set<string>();
let trabajando = false;
let liberar: number | undefined;
let quitar: (() => void) | null = null;

/** ¿Hay que (re)generar la miniatura de esta entrada con la clave `actual`? */
function necesitaMiniatura(e: Entrada, actual: string | null): boolean {
  const m = e.miniatura;
  if (!m || m.texturas === null) return true;
  return actual !== null && m.texturas !== actual;
}

/**
 * Arranca la cola: revisa la biblioteca en cuanto está cargada y cada vez que
 * cambian sus entradas (importar, deshacer un borrado…), y lo regenera todo
 * al cambiar las texturas. Devuelve la función para pararla.
 */
export function iniciarMiniaturas(): () => void {
  if (quitar) return quitar;
  const quitarBiblioteca = useBiblioteca.subscribe((s, antes) => {
    if (s.entradas !== antes.entradas || s.cargando !== antes.cargando) revisar();
  });
  const quitarTexturas = alCambiarTexturas(() => {
    generacion++;
    clave = undefined;
    descartadas.clear();
    // Rust ya las ha marcado obsoletas al guardar los ajustes; aquí lo mismo
    // en memoria, sin volver a pedir la lista. Las imágenes se siguen viendo
    // hasta que llegan las nuevas.
    useBiblioteca.setState((s) => ({
      entradas: s.entradas.map((e) =>
        e.miniatura && e.miniatura.texturas !== null
          ? { ...e, miniatura: { ...e.miniatura, texturas: null } }
          : e,
      ),
    }));
  });
  quitar = () => {
    window.clearTimeout(liberar);
    quitarBiblioteca();
    quitarTexturas();
    generacion++;
    cola.length = 0;
    enCola.clear();
    quitar = null;
  };
  revisar();
  return quitar;
}

/** Mete en la cola lo que haga falta y arranca el trabajo. */
function revisar(): void {
  if (!quitar) return;
  const { entradas, cargando } = useBiblioteca.getState();
  if (cargando) return;
  if (clave === undefined) {
    void calcularClave();
    return;
  }
  for (const e of entradas) {
    if (enCola.has(e.id) || descartadas.has(e.id) || !necesitaMiniatura(e, clave)) continue;
    cola.push(e.id);
    enCola.add(e.id);
  }
  void trabajar();
}

/** Espera al atlas activo (puede tardar la primera vez) para saber la clave. */
function calcularClave(): Promise<void> {
  if (calculando) return calculando;
  const mia = generacion;
  calculando = obtenerAtlas()
    .then(
      (a) => claveTexturas(a?.info.version ?? null),
      (e) => {
        console.warn("Miniaturas: no se pudo preparar el atlas:", e);
        return null;
      },
    )
    .then((c) => {
      if (mia === generacion) clave = c;
    })
    .finally(() => {
      calculando = null;
      revisar();
    });
  return calculando;
}

async function trabajar(): Promise<void> {
  if (trabajando) return;
  trabajando = true;
  window.clearTimeout(liberar);
  try {
    while (cola.length) {
      const id = cola.shift()!;
      enCola.delete(id);
      // Puede haberse borrado, o puesto al día, mientras esperaba.
      const e = useBiblioteca.getState().entradas.find((x) => x.id === id);
      if (!e || clave === undefined || !necesitaMiniatura(e, clave)) continue;
      await generar(id);
      await dormir(PAUSA_MS);
    }
  } finally {
    trabajando = false;
    liberar = window.setTimeout(() => {
      if (!trabajando) void cargarRender().then((m) => m.liberarRenderizador());
    }, LIBERAR_TRAS_MS);
  }
}

/** Pone la miniatura nueva en la entrada de la biblioteca (solo ese campo). */
function actualizar(id: string, miniatura: Entrada["miniatura"]): void {
  useBiblioteca.setState((s) => ({
    entradas: s.entradas.map((x) => (x.id === id ? { ...x, miniatura } : x)),
  }));
}

/** Malla → render → PNG → Rust. Un fallo deja la entrada descartada en esta sesión. */
async function generar(id: string): Promise<void> {
  const mia = generacion;
  const seguir = () =>
    mia === generacion && useBiblioteca.getState().entradas.some((x) => x.id === id);
  try {
    const malla = await mallaMiniatura(id);
    if (!seguir()) return;
    if (malla.chunks.length === 0) {
      // Nada visible: se queda el marcador plano.
      descartadas.add(id);
      return;
    }
    // El atlas tiene que ser el mismo con que se resolvieron las celdas.
    let atlas: AtlasCargado | null = null;
    if (malla.atlas) {
      const a = await obtenerAtlas(malla.atlas.version).catch(() => null);
      if (a && a.info.version === malla.atlas.version) atlas = a;
    }
    const render = await cargarRender();
    let png: Uint8Array | null;
    try {
      png = await render.renderizarMiniatura(malla, atlas, seguir);
    } catch (e) {
      render.liberarRenderizador(); // contexto perdido o roto: uno nuevo la próxima vez
      throw e;
    }
    if (!png || !seguir()) return;
    const entrada = await guardarMiniatura(id, claveTexturas(atlas?.info.version ?? null), png);
    const m = entrada.miniatura;
    // Si las texturas cambiaron mientras se guardaba, ya nace obsoleta.
    actualizar(id, m && mia !== generacion ? { ...m, texturas: null } : m);
  } catch (e) {
    descartadas.add(id);
    if ((e as Partial<ErrorApp>)?.tipo !== "noEncontrado") {
      console.warn(`Miniaturas: no se pudo generar la de ${id}:`, e);
    }
  }
}
