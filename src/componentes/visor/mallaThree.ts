// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// De la malla decodificada (src/api/visor.ts) a objetos de Three.js: un Mesh
// por chunk con los typed arrays tal cual, sin copiar, y el picking.

import {
  Box3,
  BufferAttribute,
  BufferGeometry,
  Group,
  Mesh,
  Sphere,
  Vector3,
  type Intersection,
  type Raycaster,
} from "three";
import {
  celdaImpactada,
  chunkEnCapas,
  NORMALES,
  type ChunkMalla,
  type Malla,
  type MaterialMalla,
  type Vec3,
} from "../../api/visor";
import { crearMateriales, type AtlasVisor, type MaterialesVisor, type UniformsVisor } from "./sombreado";

/** Un chunk montado: su Mesh, sus datos y su caja en coordenadas del modelo. */
export interface ChunkVisor {
  mesh: Mesh;
  chunk: ChunkMalla;
  caja: Box3;
}

/** Una malla montada en Three.js, lista para añadir a la escena. */
export interface GrupoVisor {
  grupo: Group;
  chunks: ChunkVisor[];
  materiales: MaterialesVisor;
  /** Tabla de materiales de la malla (para volver a aplicar el atlas). */
  datos: MaterialMalla[];
  dispose: () => void;
}

function geometriaDeChunk(c: ChunkMalla): BufferGeometry {
  const g = new BufferGeometry();
  // Posiciones u16 sin normalizar: el Mesh escala 1/16 (docs/visor.md 9).
  g.setAttribute("position", new BufferAttribute(c.posiciones, 3));
  g.setAttribute("normalIdx", new BufferAttribute(c.normales, 1));
  g.setAttribute("uvBloque", new BufferAttribute(c.uvs, 2));
  g.setAttribute("materialIdx", new BufferAttribute(c.materiales, 1));
  g.setIndex(new BufferAttribute(c.indices, 1));
  const total = c.indices.length;
  if (c.indicesOpacos > 0) g.addGroup(0, c.indicesOpacos, 0);
  if (total > c.indicesOpacos) g.addGroup(c.indicesOpacos, total - c.indicesOpacos, 1);
  // Caja de culling a mano: recorrer un millón de vértices para calcularla
  // costaría más que todo el montaje. En el espacio del chunk (1/16 de bloque):
  // 16 bloques en X y Z, y en Y solo las capas con geometría.
  const y0 = (c.yMin - c.origen[1]) * 16;
  const y1 = (c.yMax + 1 - c.origen[1]) * 16;
  g.boundingBox = new Box3(new Vector3(0, y0, 0), new Vector3(256, y1, 256));
  g.boundingSphere = g.boundingBox.getBoundingSphere(new Sphere());
  return g;
}

/**
 * Monta una malla. El grupo nace oculto: quien lo añade a la escena decide
 * cuándo mostrarlo (así un remallado no convive un fotograma con el anterior).
 */
export function crearGrupo(
  malla: Malla,
  comunes: UniformsVisor,
  atlas: AtlasVisor | null = null,
): GrupoVisor {
  const materiales = crearMateriales(malla.materiales, comunes, atlas);
  const lista = [materiales.opaco, materiales.translucido];
  const grupo = new Group();
  grupo.visible = false;
  const chunks = malla.chunks.map((chunk): ChunkVisor => {
    const mesh = new Mesh(geometriaDeChunk(chunk), lista);
    const [x, y, z] = chunk.origen;
    mesh.position.set(x, y, z);
    mesh.scale.setScalar(1 / 16);
    mesh.matrixAutoUpdate = false;
    mesh.updateMatrix();
    grupo.add(mesh);
    const caja = new Box3(new Vector3(x, chunk.yMin, z), new Vector3(x + 16, chunk.yMax + 1, z + 16));
    return { mesh, chunk, caja };
  });
  grupo.updateMatrixWorld(true);
  return {
    grupo,
    chunks,
    materiales,
    datos: malla.materiales,
    dispose: () => {
      for (const c of chunks) c.mesh.geometry.dispose();
      materiales.dispose();
    },
  };
}

/** Oculta los chunks que no tocan [desde, hasta]: ni se dibujan ni se pican. */
export function aplicarCorte(g: GrupoVisor, desde: number, hasta: number): void {
  for (const c of g.chunks) c.mesh.visible = chunkEnCapas(c.chunk, desde, hasta);
}

interface Impacto {
  celda: Vec3;
  distancia: number;
}

const entrada = new Vector3();
const impactos: Intersection[] = [];

/**
 * Bloque visible bajo el rayo, o null.
 *
 * No se usa un BVH (three-mesh-bvh): construirlo para millones de triángulos
 * tarda segundos y duplica el índice, y reordenaría los triángulos de los dos
 * grupos. En su lugar se ordenan los chunks visibles cuya caja corta el rayo
 * por distancia de entrada y se prueban en ese orden con el raycast de Three
 * (fuerza bruta dentro del chunk: unas decenas de miles de triángulos como
 * mucho). En cuanto el mejor impacto está más cerca que la entrada del chunk
 * siguiente se para: es el «primer impacto» sin estructura extra, y casi
 * siempre basta con uno o dos chunks.
 *
 * Los impactos en capas fuera del corte se descartan igual que el shader las
 * descarta al dibujar, así que se puede picar a través de lo cortado.
 */
export function lanzarRayo(
  g: GrupoVisor,
  raycaster: Raycaster,
  desde: number,
  hasta: number,
): Impacto | null {
  const { ray } = raycaster;
  const candidatos: { c: ChunkVisor; d: number }[] = [];
  for (const c of g.chunks) {
    if (!c.mesh.visible) continue;
    if (!ray.intersectBox(c.caja, entrada)) continue;
    candidatos.push({ c, d: entrada.distanceTo(ray.origin) });
  }
  candidatos.sort((a, b) => a.d - b.d);

  let mejor: Impacto | null = null;
  for (const { c, d } of candidatos) {
    if (mejor && d > mejor.distancia) break;
    impactos.length = 0;
    c.mesh.raycast(raycaster, impactos);
    for (const h of impactos) {
      if (mejor && h.distance >= mejor.distancia) continue;
      if (!h.face) continue;
      const normal = NORMALES[c.chunk.normales[h.face.a]] ?? NORMALES[0];
      const celda = celdaImpactada([h.point.x, h.point.y, h.point.z], normal);
      if (celda[1] < desde || celda[1] > hasta) continue;
      mejor = { celda, distancia: h.distance };
    }
  }
  impactos.length = 0;
  return mejor;
}
