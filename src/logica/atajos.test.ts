// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { describe, expect, it } from "vitest";
import { espacioBloqueadoPor } from "./atajos";

describe("espacioBloqueadoPor", () => {
  it("sin elemento con foco, el atajo vale", () => {
    expect(espacioBloqueadoPor(null)).toBeNull();
  });

  it("deja pasar el lienzo y el cuerpo", () => {
    expect(espacioBloqueadoPor({ tagName: "CANVAS" })).toBeNull();
    expect(espacioBloqueadoPor({ tagName: "BODY" })).toBeNull();
    expect(espacioBloqueadoPor({ tagName: "DIV", isContentEditable: false })).toBeNull();
  });

  it("los campos de texto y selectores se lo quedan", () => {
    expect(espacioBloqueadoPor({ tagName: "INPUT", type: "text" })).toBe("campo");
    expect(espacioBloqueadoPor({ tagName: "textarea" })).toBe("campo");
    expect(espacioBloqueadoPor({ tagName: "SELECT" })).toBe("campo");
  });

  it("las casillas y los botones se lo quedan", () => {
    expect(espacioBloqueadoPor({ tagName: "INPUT", type: "checkbox" })).toBe("campo");
    expect(espacioBloqueadoPor({ tagName: "BUTTON" })).toBe("boton");
  });

  it("el contenido editable se lo queda", () => {
    expect(espacioBloqueadoPor({ tagName: "DIV", isContentEditable: true })).toBe("editable");
  });

  it("dentro de un diálogo el atajo no vale, sea lo que sea el foco", () => {
    expect(espacioBloqueadoPor({ tagName: "DIV", dentroDeDialogo: true })).toBe("dialogo");
  });
});
