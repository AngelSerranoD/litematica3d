// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef, useState } from "react";
import { animate, useReducedMotion } from "motion/react";
import { muelle } from "../estilos/movimiento";
import { Cifra } from "./Cifra";

/**
 * Cifra que «cuenta» del valor anterior al nuevo cuando cambia. Al montarse
 * enseña el valor tal cual (las filas que entran al desplazar no cuentan) y,
 * con movimiento reducido, salta sin animar.
 */
export function CifraAnimada({ valor, className }: { valor: number; className?: string }) {
  const reducir = useReducedMotion();
  const [mostrado, setMostrado] = useState(valor);
  const actual = useRef(valor);

  useEffect(() => {
    if (reducir || actual.current === valor) {
      actual.current = valor;
      setMostrado(valor);
      return;
    }
    const control = animate(actual.current, valor, {
      ...muelle("vista"),
      onUpdate: (v) => {
        actual.current = v;
        setMostrado(Math.round(v));
      },
    });
    return () => control.stop();
  }, [valor, reducir]);

  return <Cifra valor={mostrado} className={className} />;
}
