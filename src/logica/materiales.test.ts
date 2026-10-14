// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { describe, expect, it } from "vitest";
import { type Material } from "../api/biblioteca";
import { csvLista, nombreArchivo, type Fila } from "./materiales";

const BOM = "﻿";

/** Fila con las cifras ya calculadas (las calcula el núcleo Rust; aquí solo se exportan). */
function fila(item: string, nombre: string, m: Partial<Material>): Fila {
  return {
    item,
    nombre,
    clave: "",
    cantidad: 0,
    tamanoStack: 64,
    stacks: 0,
    sueltos: 0,
    shulkers: 0,
    stacksEnShulker: 0,
    sueltosEnShulker: 0,
    ...m,
  };
}

const sinReunidos: ReadonlySet<string> = new Set();

/** Líneas de datos del CSV (sin BOM, cabecera ni fin final). */
const datos = (csv: string): string[] => csv.slice(BOM.length).split("\r\n").slice(1, -1);

describe("csvLista", () => {
  it("empieza por BOM, usa «;» y CRLF, y termina en CRLF", () => {
    const csv = csvLista([], sinReunidos);
    expect(csv).toBe(`${BOM}nombre;id;cantidad;stacks;sueltos;shulkers;reunido\r\n`);
  });

  it("escribe las cifras de ítems de 64 (stack 64, shulker 1728)", () => {
    // 1800 = 28 stacks + 8 sueltos; contiene 1 shulker llena (1728).
    const f = fila("minecraft:stone", "Stone", {
      cantidad: 1800,
      tamanoStack: 64,
      stacks: 28,
      sueltos: 8,
      shulkers: 1,
    });
    expect(datos(csvLista([f], sinReunidos))).toEqual(["Stone;minecraft:stone;1800;28;8;1;no"]);
  });

  it("escribe las cifras de ítems de 16 (stack 16, shulker 432)", () => {
    const f = fila("minecraft:ender_pearl", "Ender Pearl", {
      cantidad: 440,
      tamanoStack: 16,
      stacks: 27,
      sueltos: 8,
      shulkers: 1,
    });
    expect(datos(csvLista([f], sinReunidos))).toEqual([
      "Ender Pearl;minecraft:ender_pearl;440;27;8;1;no",
    ]);
  });

  it("escribe las cifras de ítems que no se apilan (shulker 27)", () => {
    const f = fila("minecraft:diamond_pickaxe", "Diamond Pickaxe", {
      cantidad: 30,
      tamanoStack: 1,
      stacks: 30,
      sueltos: 0,
      shulkers: 1,
    });
    expect(datos(csvLista([f], sinReunidos))).toEqual([
      "Diamond Pickaxe;minecraft:diamond_pickaxe;30;30;0;1;no",
    ]);
  });

  it("marca «sí» solo los ítems reunidos", () => {
    const a = fila("minecraft:dirt", "Dirt", { cantidad: 1 });
    const b = fila("minecraft:stone", "Stone", { cantidad: 2 });
    const csv = csvLista([a, b], new Set(["minecraft:stone"]));
    expect(datos(csv).map((l) => l.split(";").pop())).toEqual(["no", "sí"]);
  });

  it("entrecomilla los campos con «;», comillas o saltos de línea y duplica las comillas", () => {
    const l = datos(csvLista([fila("a:b", 'Uno; "dos"', {})], sinReunidos));
    expect(l[0].startsWith('"Uno; ""dos""";a:b;')).toBe(true);
    expect(csvLista([fila("a:b", "Dos\nlíneas", {})], sinReunidos)).toContain('"Dos\nlíneas";');
    expect(csvLista([fila("a:b", "x\r\ny", {})], sinReunidos)).toContain('"x\r\ny";a:b');
  });

  it("no entrecomilla la coma", () => {
    expect(datos(csvLista([fila("a:b", "Hola, mundo", {})], sinReunidos))[0]).toMatch(
      /^Hola, mundo;a:b;/,
    );
  });

  it.each(["=1+1", "+1", "-2", "@SUMA(A1)", "\tx"])(
    "neutraliza con apóstrofo el texto que empieza por %j",
    (texto) => {
      const csv = csvLista([fila("minecraft:x", texto, {})], sinReunidos);
      expect(csv).toContain(`;minecraft:x;`);
      expect(csv.slice(BOM.length).split("\r\n")[1].startsWith(`'${texto};`)).toBe(true);
    },
  );

  it("neutraliza el retorno de carro inicial (queda entrecomillado)", () => {
    const l = csvLista([fila("minecraft:x", "\rx", {})], sinReunidos);
    expect(l).toContain(`"'\rx";minecraft:x;`);
  });

  it("neutraliza también el id y lo combina con el entrecomillado", () => {
    const l = datos(csvLista([fila("=cmd;x", "Ok", {})], sinReunidos))[0];
    expect(l).toBe(`Ok;"'=cmd;x";0;0;0;0;no`);
  });

  it("no toca las cifras (aunque sean negativas) ni textos con el símbolo en medio", () => {
    const f = fila("minecraft:x", "a=b", { cantidad: -5 });
    expect(datos(csvLista([f], sinReunidos))[0]).toBe("a=b;minecraft:x;-5;0;0;0;no");
  });
});

describe("nombreArchivo", () => {
  it("quita caracteres prohibidos en Windows y colapsa espacios", () => {
    expect(nombreArchivo('a<b>:"c"/d\\e|f?g*h')).toBe("a b c d e f g h - materiales.csv");
  });

  it("usa «schematic» si el título queda vacío", () => {
    expect(nombreArchivo("  ???  ")).toBe("schematic - materiales.csv");
  });
});
