// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Previsualización de los cambios sueltos pendientes (modo Modificar): una
// capa encima de la malla con las caras de cada bloque cambiado, ya con las
// texturas del bloque nuevo. Los reemplazos de un estado por otro no pasan
// por aquí: se ven reescribiendo la tabla de materiales (sombreado.ts).

import {
  BufferAttribute,
  BufferGeometry,
  Color,
  LineSegments,
  Mesh,
  ShaderMaterial,
} from "three";
import { CARAS_ATLAS, type CeldaAtlas } from "../../api/atlas";
import type { MaterialMalla, Vec3 } from "../../api/visor";
import type { ChunkVisor, GrupoVisor } from "./mallaThree";
import { crearMateriales, type AtlasVisor, type UniformsVisor } from "./sombreado";

/** Un cambio suelto listo para dibujar. */
export interface CambioPrevia {
  /** Celda en coordenadas del modelo. */
  celda: Vec3;
  /** Estado visual que hay ahora en la celda: solo se copian sus caras. */
  estadoVisual: string;
  /** Block state nuevo. */
  estado: string;
  /** Sus caras en el atlas (orden `CARAS_ATLAS`), o `null` sin atlas. */
  caras: (CeldaAtlas | null)[] | null;
  /** Su color plano, o `null` si no se sabe. */
  color: [number, number, number] | null;
}

/** Eje (0 x, 1 y, 2 z) y sentido de las seis caras de cubo, por índice de normal. */
const EJE = [1, 1, 2, 2, 0, 0];
const POSITIVA = [true, false, false, true, false, true];

const acotar = (v: number, min: number, max: number) => Math.min(Math.max(v, min), max);
const indiceCelda = (x: number, y: number, z: number) => x + y * 16 + z * 256;

/**
 * Copia las caras de los bloques cambiados: recorre una vez cada chunk que
 * tiene cambios y, de cada quad, se queda con el trozo que cae en una celda
 * cambiada (los quads fusionados por el greedy meshing abarcan varias).
 * Mantiene la forma exacta del bloque (escaleras, losas…), que es la misma
 * en el destino porque los parecidos solo proponen bloques de su forma.
 */
function construir(g: GrupoVisor, cambios: CambioPrevia[]) {
  const porChunk = new Map<ChunkVisor, Map<number, number>>();
  for (const [i, c] of cambios.entries()) {
    const o = c.celda.map((v) => Math.floor(v / 16) * 16);
    const chunk = g.chunks.find(
      (k) => k.chunk.origen[0] === o[0] && k.chunk.origen[1] === o[1] && k.chunk.origen[2] === o[2],
    );
    if (!chunk) continue; // la celda no tiene nada dibujado
    let celdas = porChunk.get(chunk);
    if (!celdas) porChunk.set(chunk, (celdas = new Map()));
    celdas.set(indiceCelda(c.celda[0] - o[0], c.celda[1] - o[1], c.celda[2] - o[2]), i);
  }

  const posiciones: number[] = [];
  const normales: number[] = [];
  const uvs: number[] = [];
  const mats: number[] = [];
  const indices: number[] = [];
  const materiales: MaterialMalla[] = [];
  /** `cambio|cara` → fila de `materiales`. */
  const filas = new Map<string, number>();

  const material = (ci: number, original: MaterialMalla) => {
    const fi = CARAS_ATLAS.indexOf(original.cara);
    const clave = `${ci}|${fi}`;
    let fila = filas.get(clave);
    if (fila === undefined) {
      const c = cambios[ci];
      const cara = c.caras?.[fi] ?? null;
      fila = materiales.length;
      filas.set(clave, fila);
      materiales.push({
        estado: c.estado,
        nombre: c.estado,
        cara: original.cara,
        // Todo en la pasada opaca (alphaTest): la capa es pequeña y va encima.
        tipo: "recorte",
        color: cara?.color ?? c.color ?? original.color,
        textura: cara?.celda ?? null,
        rotacion: cara?.rotacion ?? 0,
      });
    }
    return fila;
  };

  const esquinas: Vec3[] = [
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
  ];
  const uvEsquinas: [number, number][] = [
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
  ];

  for (const [{ chunk: ch }, celdas] of porChunk) {
    const [ox, oy, oz] = ch.origen;
    const emitir = (ci: number, mi: number, n: number, ps: Vec3[], us: [number, number][]) => {
      const base = posiciones.length / 3;
      const fila = material(ci, g.datos[mi]);
      for (let k = 0; k < 4; k++) {
        // En 1/16 de bloque y en coordenadas del modelo (el Mesh escala 1/16).
        posiciones.push(ox * 16 + ps[k][0], oy * 16 + ps[k][1], oz * 16 + ps[k][2]);
        normales.push(n);
        uvs.push(us[k][0], us[k][1]);
        mats.push(fila);
      }
      indices.push(base, base + 1, base + 2, base, base + 2, base + 3);
    };

    // Cada quad son 6 índices seguidos [b, b+1, b+2, b, b+2, b+3] (mallado.rs).
    for (let i = 0; i + 5 < ch.indices.length; i += 6) {
      const b = ch.indices[i];
      const mi = ch.materiales[b];
      const estadoQuad = g.datos[mi]?.estado;
      if (estadoQuad === undefined) continue;
      const n = ch.normales[b];
      for (let k = 0; k < 4; k++) {
        const p = (b + k) * 3;
        esquinas[k] = [ch.posiciones[p], ch.posiciones[p + 1], ch.posiciones[p + 2]];
        uvEsquinas[k] = [ch.uvs[(b + k) * 2], ch.uvs[(b + k) * 2 + 1]];
      }

      if (n >= 6) {
        // Planos cruzados: un quad por bloque; cuenta su centro.
        const centro = [0, 1, 2].map(
          (a) => (esquinas[0][a] + esquinas[1][a] + esquinas[2][a] + esquinas[3][a]) / 4,
        );
        const ci = celdas.get(
          indiceCelda(Math.floor(centro[0] / 16), Math.floor(centro[1] / 16), Math.floor(centro[2] / 16)),
        );
        if (ci !== undefined && cambios[ci].estadoVisual === estadoQuad)
          emitir(ci, mi, n, esquinas, uvEsquinas);
        continue;
      }

      // Cara de cubo o caja: el bloque es el de detrás de la cara (se
      // retrocede contra la normal, como en el picking).
      const e = EJE[n];
      const ue = (e + 1) % 3;
      const ve = (e + 2) % 3;
      const celdaE = Math.floor((esquinas[0][e] - (POSITIVA[n] ? 0.01 : -0.01)) / 16);
      if (celdaE < 0 || celdaE > 15) continue;
      let u0 = Infinity;
      let u1 = -Infinity;
      let v0 = Infinity;
      let v1 = -Infinity;
      for (const p of esquinas) {
        u0 = Math.min(u0, p[ue]);
        u1 = Math.max(u1, p[ue]);
        v0 = Math.min(v0, p[ve]);
        v1 = Math.max(v1, p[ve]);
      }
      if (u1 <= u0 || v1 <= v0) continue;
      for (let cv = Math.floor(v0 / 16); cv * 16 < v1; cv++) {
        for (let cu = Math.floor(u0 / 16); cu * 16 < u1; cu++) {
          const celda: Vec3 = [0, 0, 0];
          celda[e] = celdaE;
          celda[ue] = cu;
          celda[ve] = cv;
          const ci = celdas.get(indiceCelda(celda[0], celda[1], celda[2]));
          if (ci === undefined || cambios[ci].estadoVisual !== estadoQuad) continue;
          // Recorte a la celda: cada esquina se acota (el rectángulo y su
          // orden se conservan) y su UV se interpola, porque las UV son
          // afines en la posición dentro del quad.
          const a0 = Math.max(u0, cu * 16);
          const a1 = Math.min(u1, cu * 16 + 16);
          const b0 = Math.max(v0, cv * 16);
          const b1 = Math.min(v1, cv * 16 + 16);
          const ps: Vec3[] = [];
          const us: [number, number][] = [];
          for (const p of esquinas) {
            const q: Vec3 = [p[0], p[1], p[2]];
            q[ue] = acotar(p[ue], a0, a1);
            q[ve] = acotar(p[ve], b0, b1);
            const s = (q[ue] - u0) / (u1 - u0);
            const t = (q[ve] - v0) / (v1 - v0);
            let uu = 0;
            let vv = 0;
            for (let k = 0; k < 4; k++) {
              const ws = esquinas[k][ue] === u1 ? s : 1 - s;
              const wt = esquinas[k][ve] === v1 ? t : 1 - t;
              uu += ws * wt * uvEsquinas[k][0];
              vv += ws * wt * uvEsquinas[k][1];
            }
            ps.push(q);
            us.push([uu, vv]);
          }
          emitir(ci, mi, n, ps, us);
        }
      }
    }
  }
  return { posiciones, normales, uvs, mats, indices, materiales };
}

const verticeMarco = /* glsl */ `
attribute float capa;
varying float vCapa;
void main() {
  vCapa = capa;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const fragmentoMarco = /* glsl */ `
uniform float uDesde;
uniform float uHasta;
uniform vec3 uColor;
varying float vCapa;
void main() {
  // Mismo corte por capas que la malla: sin marcos flotando sobre lo cortado.
  if (vCapa < uDesde || vCapa > uHasta) discard;
  gl_FragColor = vec4(uColor, 1.0);
}
`;

/** Aristas de un cubo unidad: pares de esquinas (bits x, y, z). */
const ARISTAS = [
  [0, 1], [2, 3], [4, 5], [6, 7], // a lo largo de x
  [0, 2], [1, 3], [4, 6], [5, 7], // a lo largo de y
  [0, 4], [1, 5], [2, 6], [3, 7], // a lo largo de z
];

/** Marco de alambre de cada celda cambiada: dice «aquí hay un cambio pendiente». */
function crearMarcos(cambios: CambioPrevia[], comunes: UniformsVisor, color: Color) {
  const m = 0.012; // un poco más que la caja de selección (1,01): sin parpadeo
  const pos = new Float32Array(cambios.length * ARISTAS.length * 6);
  const capas = new Float32Array(cambios.length * ARISTAS.length * 2);
  let p = 0;
  let q = 0;
  for (const { celda } of cambios) {
    const esquina = (bits: number) => [
      celda[0] + (bits & 1 ? 1 + m : -m),
      celda[1] + (bits & 2 ? 1 + m : -m),
      celda[2] + (bits & 4 ? 1 + m : -m),
    ];
    for (const [a, b] of ARISTAS) {
      pos.set(esquina(a), p);
      pos.set(esquina(b), p + 3);
      p += 6;
      capas[q++] = celda[1];
      capas[q++] = celda[1];
    }
  }
  const geometria = new BufferGeometry();
  geometria.setAttribute("position", new BufferAttribute(pos, 3));
  geometria.setAttribute("capa", new BufferAttribute(capas, 1));
  geometria.computeBoundingSphere();
  const material = new ShaderMaterial({
    vertexShader: verticeMarco,
    fragmentShader: fragmentoMarco,
    uniforms: { uDesde: comunes.uDesde, uHasta: comunes.uHasta, uColor: { value: color } },
    toneMapped: false,
  });
  const lineas = new LineSegments(geometria, material);
  lineas.renderOrder = 9;
  return lineas;
}

/**
 * Monta la capa de los cambios sueltos dentro del grupo de una malla (se ve
 * y se oculta con él, y no entra en el picking, que solo mira los chunks).
 * Devuelve la función que la quita y libera.
 */
export function montarCambios(
  g: GrupoVisor,
  cambios: CambioPrevia[],
  comunes: UniformsVisor,
  atlas: AtlasVisor | null,
  colorMarco: Color,
): () => void {
  if (!cambios.length) return () => {};
  const d = construir(g, cambios);
  const quitar: (() => void)[] = [];

  if (d.indices.length) {
    const geometria = new BufferGeometry();
    geometria.setAttribute("position", new BufferAttribute(new Float32Array(d.posiciones), 3));
    geometria.setAttribute("normalIdx", new BufferAttribute(new Float32Array(d.normales), 1));
    geometria.setAttribute("uvBloque", new BufferAttribute(new Float32Array(d.uvs), 2));
    geometria.setAttribute("materialIdx", new BufferAttribute(new Float32Array(d.mats), 1));
    geometria.setIndex(new BufferAttribute(new Uint32Array(d.indices), 1));
    geometria.computeBoundingSphere();
    const materiales = crearMateriales(d.materiales, comunes, atlas);
    // Mismas posiciones que las caras de debajo: gana la capa por desplazamiento de profundidad.
    materiales.opaco.polygonOffset = true;
    materiales.opaco.polygonOffsetFactor = -1;
    materiales.opaco.polygonOffsetUnits = -4;
    const mesh = new Mesh(geometria, materiales.opaco);
    mesh.scale.setScalar(1 / 16);
    mesh.updateMatrix();
    mesh.matrixAutoUpdate = false;
    g.grupo.add(mesh);
    mesh.updateMatrixWorld(true);
    quitar.push(() => {
      g.grupo.remove(mesh);
      geometria.dispose();
      materiales.dispose();
    });
  }

  const marcos = crearMarcos(cambios, comunes, colorMarco);
  g.grupo.add(marcos);
  marcos.updateMatrixWorld(true);
  quitar.push(() => {
    g.grupo.remove(marcos);
    marcos.geometry.dispose();
    (marcos.material as ShaderMaterial).dispose();
  });

  return () => quitar.forEach((q) => q());
}
