// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { describe, expect, it } from "vitest";
import { decodificarMalla } from "./visor";

const alinear4 = (n: number) => (n + 3) & ~3;

interface ChunkPrueba {
  origen: [number, number, number];
  yMin: number;
  yMax: number;
  posiciones: number[];
  normales: number[];
  uvs: number[];
  materiales: number[];
  indices: number[];
  indicesOpacos: number;
  bytesIndice: 2 | 4;
}

/** Bytes de datos de un chunk (sin su cabecera de 40). */
function datosChunk(c: ChunkPrueba): number {
  const nv = c.normales.length;
  let n = alinear4(nv * 6);
  n = alinear4(n + nv);
  n += nv * 4;
  n = alinear4(n + nv * 2);
  return alinear4(n + c.indices.length * c.bytesIndice);
}

/** Construye un binario de malla con el formato de docs/visor.md. */
function construir(
  chunks: ChunkPrueba[],
  opciones: { magia?: string; version?: number } = {},
): ArrayBuffer {
  const json = new TextEncoder().encode(
    JSON.stringify({
      materiales: [
        { estado: "minecraft:stone", nombre: "minecraft:stone", cara: "arriba", tipo: "opaco" },
      ],
      paleta: ["minecraft:air", "minecraft:stone"],
      totalBloques: 7,
    }),
  );
  const tamCabecera = 44;
  const inicio = alinear4(tamCabecera + json.length);
  const total = chunks.reduce((s, c) => s + 40 + datosChunk(c), inicio);
  const buffer = new ArrayBuffer(total);
  const dv = new DataView(buffer);
  const magia = opciones.magia ?? "L3DM";
  for (let i = 0; i < 4; i++) dv.setUint8(i, magia.charCodeAt(i));
  dv.setUint32(4, opciones.version ?? 1, true);
  dv.setUint32(8, tamCabecera, true);
  dv.setUint32(12, chunks.length, true);
  [-1, 2, -3].forEach((v, i) => dv.setInt32(16 + i * 4, v, true));
  [4, 5, 6].forEach((v, i) => dv.setUint32(28 + i * 4, v, true));
  dv.setUint32(40, json.length, true);
  new Uint8Array(buffer, tamCabecera, json.length).set(json);

  let o = inicio;
  for (const c of chunks) {
    const nv = c.normales.length;
    c.origen.forEach((v, i) => dv.setInt32(o + i * 4, v, true));
    dv.setInt32(o + 12, c.yMin, true);
    dv.setInt32(o + 16, c.yMax, true);
    dv.setUint32(o + 20, nv, true);
    dv.setUint32(o + 24, c.indices.length, true);
    dv.setUint32(o + 28, c.indicesOpacos, true);
    dv.setUint32(o + 32, c.bytesIndice, true);
    dv.setUint32(o + 36, datosChunk(c), true);
    let p = o + 40;
    c.posiciones.forEach((v, i) => dv.setUint16(p + i * 2, v, true));
    p = alinear4(p + nv * 6);
    c.normales.forEach((v, i) => dv.setUint8(p + i, v));
    p = alinear4(p + nv);
    c.uvs.forEach((v, i) => dv.setUint16(p + i * 2, v, true));
    p += nv * 4;
    c.materiales.forEach((v, i) => dv.setUint16(p + i * 2, v, true));
    p = alinear4(p + nv * 2);
    c.indices.forEach((v, i) => {
      if (c.bytesIndice === 2) dv.setUint16(p + i * 2, v, true);
      else dv.setUint32(p + i * 4, v, true);
    });
    o += 40 + datosChunk(c);
  }
  return buffer;
}

// 3 vértices (impar, para probar la alineación a 4 bytes).
const chunkA: ChunkPrueba = {
  origen: [0, 1, -1],
  yMin: -4,
  yMax: 9,
  posiciones: [1, 2, 3, 4, 5, 6, 7, 8, 9],
  normales: [0, 3, 5],
  uvs: [10, 11, 12, 13, 14, 15],
  materiales: [0, 0, 0],
  indices: [0, 1, 2],
  indicesOpacos: 3,
  bytesIndice: 2,
};
const chunkB: ChunkPrueba = {
  origen: [2, 0, 3],
  yMin: 0,
  yMax: 1,
  posiciones: [100, 200, 300, 400, 500, 600],
  normales: [1, 4],
  uvs: [1, 2, 3, 4],
  materiales: [0, 0],
  indices: [0, 1, 70000],
  indicesOpacos: 1,
  bytesIndice: 4,
};

describe("decodificarMalla", () => {
  it("lee cabecera y JSON (rotación por defecto 0, sin atlas)", () => {
    const m = decodificarMalla(construir([]));
    expect(m.origen).toEqual([-1, 2, -3]);
    expect(m.tamano).toEqual([4, 5, 6]);
    expect(m.paleta).toEqual(["minecraft:air", "minecraft:stone"]);
    expect(m.totalBloques).toBe(7);
    expect(m.atlas).toBeNull();
    expect(m.materiales[0].rotacion).toBe(0);
    expect(m.chunks).toEqual([]);
  });

  it("decodifica los arrays de varios chunks (índices u16 y u32, con relleno)", () => {
    const m = decodificarMalla(construir([chunkA, chunkB]));
    expect(m.chunks).toHaveLength(2);
    const [a, b] = m.chunks;
    expect(a.origen).toEqual([0, 1, -1]);
    expect([a.yMin, a.yMax, a.indicesOpacos]).toEqual([-4, 9, 3]);
    expect(Array.from(a.posiciones)).toEqual(chunkA.posiciones);
    expect(Array.from(a.normales)).toEqual(chunkA.normales);
    expect(Array.from(a.uvs)).toEqual(chunkA.uvs);
    expect(Array.from(a.materiales)).toEqual(chunkA.materiales);
    expect(a.indices).toBeInstanceOf(Uint16Array);
    expect(Array.from(a.indices)).toEqual([0, 1, 2]);
    expect(b.origen).toEqual([2, 0, 3]);
    expect(Array.from(b.posiciones)).toEqual(chunkB.posiciones);
    expect(Array.from(b.normales)).toEqual([1, 4]);
    expect(Array.from(b.uvs)).toEqual([1, 2, 3, 4]);
    expect(b.indices).toBeInstanceOf(Uint32Array);
    expect(Array.from(b.indices)).toEqual([0, 1, 70000]);
  });

  it("los arrays comparten el ArrayBuffer original (sin copiar)", () => {
    const buf = construir([chunkA]);
    expect(decodificarMalla(buf).chunks[0].posiciones.buffer).toBe(buf);
  });

  it("rechaza una magia incorrecta", () => {
    expect(() => decodificarMalla(construir([], { magia: "XXXX" }))).toThrow(/formato válido/);
  });

  it("rechaza una versión no soportada", () => {
    expect(() => decodificarMalla(construir([], { version: 2 }))).toThrow(/Versión de malla 2/);
  });

  it("lanza con un buffer vacío", () => {
    expect(() => decodificarMalla(new ArrayBuffer(0))).toThrow();
  });

  it("lanza con un buffer truncado (cabecera o datos de chunk incompletos)", () => {
    const buf = construir([chunkA, chunkB]);
    expect(() => decodificarMalla(buf.slice(0, 20))).toThrow();
    expect(() => decodificarMalla(buf.slice(0, buf.byteLength - 8))).toThrow();
  });
});
