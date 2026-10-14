// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Render fuera de pantalla de la miniatura isométrica de un schematic
// (docs/miniaturas.md). Usa la misma malla y los mismos materiales que el
// visor (mallaThree.ts y sombreado.ts) con un WebGLRenderer propio sobre un
// lienzo que nunca entra en el DOM. Va en el trozo de Three: se importa de
// forma dinámica desde la cola de miniaturas.

import { OrthographicCamera, Scene, Vector3, WebGLRenderer } from "three";
import type { AtlasCargado } from "../../api/atlas";
import type { Malla } from "../../api/visor";
import { dormir } from "../../logica/tiempo";
import { crearAtlasVisor } from "./atlasThree";
import { crearGrupo, type ChunkVisor } from "./mallaThree";
import { crearUniformsVisor, type AtlasVisor } from "./sombreado";

/** Lado del PNG en píxeles (Rust admite de 64 a 1024, cuadrado). */
const LADO_MINIATURA = 512;

/**
 * Vértices que se suben a la GPU y se dibujan de una vez antes de devolver
 * el hilo a la interfaz. Un schematic enorme se dibuja en muchas tandas sobre
 * el mismo lienzo (sin borrar entre ellas): nunca congela la ventana.
 */
const VERTICES_POR_TANDA = 300_000;

/** Aire alrededor del modelo, en fracción del lado. */
const MARGEN = 0.04;

/**
 * Cámara isométrica desde +X +Y +Z: la misma vista que el marcador plano
 * (Bloque.tsx), con las caras de arriba, la de Z = largo y la de X = ancho.
 */
const DIRECCION = new Vector3(1, 1, 1).normalize();

let renderizador: WebGLRenderer | null = null;
/** Textura del atlas propia de este renderizador (no la del visor). */
let atlasPropio: { imagen: ImageBitmap; atlas: AtlasVisor } | null = null;

/** El único renderizador de miniaturas, creado la primera vez. */
function obtenerRenderizador(): WebGLRenderer {
  if (renderizador) return renderizador;
  const lienzo = document.createElement("canvas");
  lienzo.width = LADO_MINIATURA;
  lienzo.height = LADO_MINIATURA;
  const r = new WebGLRenderer({
    canvas: lienzo,
    alpha: true,
    antialias: true,
    // El dibujo se hace en varias tandas y se lee al final con toBlob.
    preserveDrawingBuffer: true,
    powerPreference: "low-power",
  });
  r.setPixelRatio(1);
  r.setSize(LADO_MINIATURA, LADO_MINIATURA, false);
  r.setClearColor(0x000000, 0); // fondo transparente
  r.autoClear = false;
  renderizador = r;
  return r;
}

function atlasDe(a: AtlasCargado): AtlasVisor {
  if (atlasPropio?.imagen === a.imagen) return atlasPropio.atlas;
  atlasPropio?.atlas.textura.dispose();
  const atlas = crearAtlasVisor(a);
  atlasPropio = { imagen: a.imagen, atlas };
  return atlas;
}

/**
 * Suelta el contexto WebGL y la textura del atlas. La cola lo llama cuando se
 * queda sin trabajo; la siguiente miniatura vuelve a crearlos.
 */
export function liberarRenderizador(): void {
  atlasPropio?.atlas.textura.dispose();
  atlasPropio = null;
  if (!renderizador) return;
  renderizador.dispose();
  renderizador.forceContextLoss();
  renderizador = null;
}

/** Cámara ortográfica que encuadra la caja del modelo, cuadrada y centrada. */
function encuadrar(camara: OrthographicCamera, malla: Malla): void {
  const [ancho, alto, largo] = malla.tamano;
  const centro = new Vector3(ancho / 2, alto / 2, largo / 2);
  const radio = Math.max(Math.hypot(ancho, alto, largo) / 2, 1);
  camara.position.copy(centro).addScaledVector(DIRECCION, radio * 2);
  camara.up.set(0, 1, 0);
  camara.lookAt(centro);
  camara.updateMatrixWorld();

  // Las 8 esquinas en el espacio de la cámara dan el rectángulo que ocupa.
  const p = new Vector3();
  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;
  let minZ = Infinity;
  let maxZ = -Infinity;
  for (const x of [0, ancho])
    for (const y of [0, alto])
      for (const z of [0, largo]) {
        p.set(x, y, z).applyMatrix4(camara.matrixWorldInverse);
        minX = Math.min(minX, p.x);
        maxX = Math.max(maxX, p.x);
        minY = Math.min(minY, p.y);
        maxY = Math.max(maxY, p.y);
        minZ = Math.min(minZ, p.z);
        maxZ = Math.max(maxZ, p.z);
      }
  const medio = (Math.max(maxX - minX, maxY - minY, 1) / 2) * (1 + 2 * MARGEN);
  const cx = (minX + maxX) / 2;
  const cy = (minY + maxY) / 2;
  camara.left = cx - medio;
  camara.right = cx + medio;
  camara.top = cy + medio;
  camara.bottom = cy - medio;
  // La cámara mira hacia −Z: lo más cercano tiene la Z mayor.
  camara.near = -maxZ - 1;
  camara.far = -minZ + 1;
  camara.updateProjectionMatrix();
}

/** Distancia de un chunk a la cámara a lo largo de la vista (mayor = más lejos). */
const lejania = (c: ChunkVisor) => {
  const [x, , z] = c.chunk.origen;
  const y = (c.chunk.yMin + c.chunk.yMax + 1) / 2;
  return -((x + 8) * DIRECCION.x + y * DIRECCION.y + (z + 8) * DIRECCION.z);
};

/**
 * Dibuja la miniatura de una malla y devuelve el PNG (fondo transparente,
 * LADO_MINIATURA de lado). `seguir` se consulta entre tandas: si devuelve
 * false (cambiaron las texturas, se borró el schematic…) se abandona y
 * devuelve `null`. Geometrías y materiales se liberan siempre al acabar.
 */
export async function renderizarMiniatura(
  malla: Malla,
  atlas: AtlasCargado | null,
  seguir: () => boolean,
): Promise<Uint8Array | null> {
  const r = obtenerRenderizador();
  const grupo = crearGrupo(malla, crearUniformsVisor(), atlas ? atlasDe(atlas) : null);
  const escena = new Scene();
  escena.add(grupo.grupo);
  grupo.grupo.visible = true;
  const camara = new OrthographicCamera();
  encuadrar(camara, malla);
  const { opaco, translucido } = grupo.materiales;

  /** Dibuja los chunks de `lista` en tandas, cediendo el hilo entre ellas. */
  const dibujar = async (lista: ChunkVisor[]): Promise<boolean> => {
    let i = 0;
    while (i < lista.length) {
      let vertices = 0;
      const tanda: ChunkVisor[] = [];
      while (i < lista.length && (tanda.length === 0 || vertices < VERTICES_POR_TANDA)) {
        tanda.push(lista[i]);
        vertices += lista[i].chunk.normales.length;
        i++;
      }
      for (const c of tanda) c.mesh.visible = true;
      r.render(escena, camara);
      for (const c of tanda) c.mesh.visible = false;
      await dormir(0);
      if (!seguir()) return false;
    }
    return true;
  };

  try {
    r.setRenderTarget(null);
    r.clear(true, true, true);
    for (const c of grupo.chunks) c.mesh.visible = false;

    // 1.ª pasada: opacos y de recorte, con profundidad.
    opaco.visible = true;
    translucido.visible = false;
    const conOpacos = grupo.chunks.filter((c) => c.chunk.indicesOpacos > 0);
    if (!(await dibujar(conOpacos))) return null;
    // Lo que no tiene nada translúcido ya no hace falta en la GPU.
    for (const c of grupo.chunks) {
      if (c.chunk.indices.length === c.chunk.indicesOpacos) c.mesh.geometry.dispose();
    }

    // 2.ª pasada: translúcidos (agua, cristal…) del más lejano al más cercano,
    // sobre la profundidad que dejaron los opacos.
    opaco.visible = false;
    translucido.visible = true;
    const conTranslucidos = grupo.chunks
      .filter((c) => c.chunk.indices.length > c.chunk.indicesOpacos)
      .sort((a, b) => lejania(b) - lejania(a));
    if (!(await dibujar(conTranslucidos))) return null;

    if (r.getContext().isContextLost()) throw new Error("Se perdió el contexto WebGL.");
    const blob = await new Promise<Blob | null>((ok) =>
      r.domElement.toBlob(ok, "image/png"),
    );
    if (!blob) throw new Error("No se pudo codificar la miniatura en PNG.");
    return new Uint8Array(await blob.arrayBuffer());
  } finally {
    escena.remove(grupo.grupo);
    grupo.dispose();
    r.renderLists.dispose();
  }
}
