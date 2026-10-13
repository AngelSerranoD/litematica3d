// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Colores de la escena 3D sacados de tokens.css, para que el visor siga al
// tema claro u oscuro igual que el resto de la interfaz.

import { useEffect, useState } from "react";
import { Color, SRGBColorSpace } from "three";

interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

const RGB = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:\s*[/,]\s*([\d.]+)(%?))?\s*\)$/i;

/** Interpreta `#rgb`, `#rrggbb` o `rgb(r g b / a)` (lo que hay en tokens.css). */
function interpretar(valor: string): Rgba {
  const v = valor.trim();
  if (v.startsWith("#")) {
    const hex = v.length === 4 ? v.slice(1).split("").map((c) => c + c).join("") : v.slice(1, 7);
    const n = Number.parseInt(hex, 16);
    return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255, a: 1 };
  }
  const m = RGB.exec(v);
  if (m) {
    const alfa = m[4] === undefined ? 1 : Number(m[4]) / (m[5] ? 100 : 1);
    return { r: Number(m[1]), g: Number(m[2]), b: Number(m[3]), a: alfa };
  }
  throw new Error(`Color de token no reconocido: «${valor}»`);
}

function token(nombre: string): Rgba {
  return interpretar(getComputedStyle(document.documentElement).getPropertyValue(nombre));
}

/**
 * Color de un token. Si tiene transparencia (los filetes), se compone sobre
 * `fondo`: la rejilla 3D no mezcla con lo que hay detrás, así que se le da el
 * color que se vería en pantalla.
 */
export function colorToken(nombre: string, fondo = "--superficie"): Color {
  const c = token(nombre);
  const f = c.a < 1 ? token(fondo) : c;
  const mezcla = (x: number, y: number) => (x * c.a + y * (1 - c.a)) / 255;
  return new Color().setRGB(mezcla(c.r, f.r), mezcla(c.g, f.g), mezcla(c.b, f.b), SRGBColorSpace);
}

/** Igual que colorToken, pero como `#rrggbb` (para props que piden cadena). */
export const hexToken = (nombre: string, fondo?: string) =>
  `#${colorToken(nombre, fondo).getHexString(SRGBColorSpace)}`;

/**
 * Número que cambia cada vez que cambia el tema (preferencia del sistema o
 * `data-tema` en <html>): sirve de dependencia para recalcular colores.
 */
export function usoTema(): number {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    const avisar = () => setVersion((v) => v + 1);
    const oscuro = window.matchMedia("(prefers-color-scheme: dark)");
    oscuro.addEventListener("change", avisar);
    const observador = new MutationObserver(avisar);
    observador.observe(document.documentElement, { attributes: true, attributeFilter: ["data-tema"] });
    return () => {
      oscuro.removeEventListener("change", avisar);
      observador.disconnect();
    };
  }, []);
  return version;
}
