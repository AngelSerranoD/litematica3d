// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { memo, useMemo, useState, type CSSProperties } from "react";
import { m } from "motion/react";
import type { Dimensiones, Miniatura as DatosMiniatura } from "../api/biblioteca";
import { urlMiniatura } from "../api/miniaturas";
import { muelle } from "../estilos/movimiento";
import { Bloque } from "./Bloque";
import "./Miniatura.css";

/** Los cinco colores de la paleta (variables --paleta-* de tokens.css). */
const TONOS = ["hierba", "salvia", "musgo", "oliva", "tierra"] as const;

/** FNV-1a de 32 bits: el mismo id da siempre el mismo color. */
function hash(texto: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < texto.length; i++) {
    h ^= texto.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/**
 * Proporciones del dibujo: las reales, pero ningún lado baja del 12 % del
 * mayor (una losa de 200 × 1 × 200 se vería como una línea).
 */
function proporciones({ ancho, alto, largo }: Dimensiones): Dimensiones {
  const mayor = Math.max(ancho, alto, largo, 1);
  const minimo = mayor * 0.12;
  return {
    ancho: Math.max(ancho, minimo),
    alto: Math.max(alto, minimo),
    largo: Math.max(largo, minimo),
  };
}

interface Props {
  id: string;
  dimensiones: Dimensiones;
  /** Miniatura renderizada (`Entrada.miniatura`); sin ella, el marcador plano. */
  miniatura?: DatosMiniatura | null;
  /** layoutId del dibujo (elemento compartido biblioteca ↔ schematic). */
  layoutId?: string;
}

/**
 * Miniatura de un schematic: el render isométrico guardado en la biblioteca
 * (docs/miniaturas.md) o, mientras no existe o si no se puede leer, un prisma
 * isométrico de colores planos con las proporciones de sus medidas y un tono
 * de la paleta derivado del id. Las dos llevan el mismo layoutId.
 */
export const Miniatura = memo(function Miniatura({ id, dimensiones, miniatura, layoutId }: Props) {
  const { ancho, alto, largo } = dimensiones;
  const dibujo = useMemo(() => proporciones({ ancho, alto, largo }), [ancho, alto, largo]);
  const estilo = useMemo(
    () => ({ "--bloque-base": `var(--paleta-${TONOS[hash(id) % TONOS.length]})` }) as CSSProperties,
    [id],
  );
  const ruta = miniatura?.ruta;
  const fecha = miniatura?.fecha;
  const url = useMemo(
    () => (ruta !== undefined && fecha !== undefined ? urlMiniatura({ ruta, fecha, texturas: null }) : null),
    [ruta, fecha],
  );
  // URL que no se pudo cargar (archivo borrado a mano…): vuelve el marcador.
  const [rota, setRota] = useState<string | null>(null);

  if (url && url !== rota) {
    const imagen = {
      className: "miniatura-imagen",
      src: url,
      alt: "",
      draggable: false,
      decoding: "async" as const,
      onError: () => setRota(url),
    };
    return layoutId ? (
      <m.img {...imagen} layoutId={layoutId} transition={{ layout: muelle("compartido") }} />
    ) : (
      <img {...imagen} loading="lazy" />
    );
  }
  return <Bloque dimensiones={dibujo} layoutId={layoutId} className="miniatura" style={estilo} />;
});
