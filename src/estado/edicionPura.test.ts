// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { describe, expect, it } from "vitest";
import { agruparMiembros, bloquesAfectados, inversaDe } from "./edicionPura";

const PIEDRA = "minecraft:stone";
const roca = (cantidad: number | null, destino = "minecraft:andesite") => ({
  estadoOrigen: PIEDRA,
  estadoDestino: destino,
  cantidad,
});

describe("bloquesAfectados", () => {
  it("suma cambios sueltos y reemplazos sin solaparse", () => {
    const r = bloquesAfectados([{ estadoAnterior: "minecraft:dirt" }], [roca(10)]);
    expect(r).toEqual({ total: 11, exacto: true });
  });

  it("no cuenta dos veces un cambio suelto que cae dentro de un reemplazo", () => {
    const r = bloquesAfectados(
      [{ estadoAnterior: PIEDRA }, { estadoAnterior: PIEDRA }, { estadoAnterior: "minecraft:dirt" }],
      [roca(661)],
    );
    expect(r).toEqual({ total: 662, exacto: true }); // 661 + el de tierra
  });

  it("un reemplazo sin cantidad no suma y deja el total inexacto", () => {
    const r = bloquesAfectados([{ estadoAnterior: PIEDRA }], [roca(null)]);
    expect(r).toEqual({ total: 1, exacto: false });
  });

  it("un reemplazo de un estado por sí mismo no absorbe cambios", () => {
    const r = bloquesAfectados([{ estadoAnterior: PIEDRA }], [roca(5, PIEDRA)]);
    expect(r.total).toBe(6);
  });
});

describe("agruparMiembros", () => {
  it("junta escaleras con distinto facing y suma cantidades", () => {
    const g = agruparMiembros([
      { estado: "minecraft:oak_stairs[facing=north,half=bottom]", cantidad: 4 },
      { estado: "minecraft:oak_planks", cantidad: 7 },
      { estado: "minecraft:oak_stairs[facing=south,half=bottom]", cantidad: 6 },
    ]);
    expect(g).toHaveLength(2);
    expect(g[0].id).toBe("minecraft:oak_stairs");
    expect(g[0].cantidad).toBe(10);
    expect(g[0].estados).toHaveLength(2);
    expect(g[1]).toMatchObject({ id: "minecraft:oak_planks", cantidad: 7 });
  });
});

describe("inversaDe", () => {
  it("cada acción ofrece su inversa", () => {
    expect(inversaDe("Deshacer")).toBe("Rehacer");
    expect(inversaDe("Rehacer")).toBe("Deshacer");
  });
});
