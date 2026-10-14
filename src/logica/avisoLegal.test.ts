// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { describe, expect, it } from "vitest";
import { AVISO_MARCA } from "./avisoLegal";
import ajustes from "../componentes/PanelAjustes.tsx?raw";
import readme from "../../README.md?raw";

describe("aviso de marca", () => {
  it("dice lo acordado", () => {
    expect(AVISO_MARCA).toBe(
      "No es un producto oficial de Minecraft. No está aprobado por Mojang ni Microsoft ni asociado con ellos.",
    );
  });

  it("aparece en Ajustes y en el README", () => {
    expect(ajustes).toContain("{AVISO_MARCA}");
    expect(readme).toContain(AVISO_MARCA);
  });
});
