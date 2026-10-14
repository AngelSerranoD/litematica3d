// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { CSSProperties } from "react";
import { m } from "motion/react";
import type { Dimensiones } from "../api/biblioteca";
import { muelle } from "../estilos/movimiento";
import "./Bloque.css";

const COS30 = Math.cos(Math.PI / 6);

type Punto = readonly [number, number, number];

/** Proyección isométrica: X hacia la derecha-abajo, Z hacia la izquierda-abajo, Y arriba. */
function proyectar([x, y, z]: Punto): [number, number] {
  return [(x - z) * COS30, (x + z) * 0.5 - y];
}

function trazo(puntos: Punto[]): string {
  return puntos.map((p) => proyectar(p).map((v) => v.toFixed(3)).join(",")).join(" ");
}

interface Props {
  dimensiones: Dimensiones;
  /** Si se da, el dibujo es un elemento compartido entre vistas. */
  layoutId?: string;
  /**
   * Clase extra; con ella se pueden redefinir los colores de las caras
   * (--bloque-superior, --bloque-frontal, --bloque-lateral, --bloque-arista).
   */
  className?: string;
  style?: CSSProperties;
}

/**
 * Prisma isométrico con las proporciones del schematic: marcador plano hasta
 * que exista el visor 3D. Caras con relleno liso y aristas de 1 px.
 */
export function Bloque({ dimensiones, layoutId, className, style }: Props) {
  const { ancho: a, alto: h, largo: l } = dimensiones;

  // Caras visibles desde +X +Z: la superior, la de Z = largo y la de X = ancho.
  const superior: Punto[] = [[0, h, 0], [a, h, 0], [a, h, l], [0, h, l]];
  const frontal: Punto[] = [[0, 0, l], [a, 0, l], [a, h, l], [0, h, l]];
  const lateral: Punto[] = [[a, 0, 0], [a, 0, l], [a, h, l], [a, h, 0]];

  const todos = [...superior, ...frontal, ...lateral].map(proyectar);
  const xs = todos.map(([x]) => x);
  const ys = todos.map(([, y]) => y);
  const minX = Math.min(...xs);
  const minY = Math.min(...ys);
  const ancho = Math.max(...xs) - minX;
  const alto = Math.max(...ys) - minY;
  const margen = Math.max(ancho, alto) * 0.04;
  const viewBox = [minX - margen, minY - margen, ancho + 2 * margen, alto + 2 * margen]
    .map((v) => v.toFixed(3))
    .join(" ");

  const clase = className ? `bloque ${className}` : "bloque";
  const dibujo = (
    <svg viewBox={viewBox} preserveAspectRatio="xMidYMid meet" aria-hidden="true">
      <polygon className="bloque__cara bloque__cara--superior" points={trazo(superior)} />
      <polygon className="bloque__cara bloque__cara--frontal" points={trazo(frontal)} />
      <polygon className="bloque__cara bloque__cara--lateral" points={trazo(lateral)} />
    </svg>
  );

  // Sin layoutId basta un div: cada componente de Motion cuesta, y en la
  // biblioteca hay cientos de miniaturas.
  if (!layoutId) {
    return (
      <div className={clase} style={style}>
        {dibujo}
      </div>
    );
  }

  // El layoutId va en un div: Motion no proyecta layout sobre elementos SVG.
  return (
    <m.div
      className={clase}
      style={style}
      layoutId={layoutId}
      transition={{ layout: muelle("compartido") }}
    >
      {dibujo}
    </m.div>
  );
}
