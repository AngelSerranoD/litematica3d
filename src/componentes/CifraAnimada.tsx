// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef, useState } from "react";
import { animate, useReducedMotion } from "motion/react";
import { Cifra } from "./Cifra";

/** Duración del conteo: la de --dur-media (0 con movimiento reducido). */
function duracionConteo(): number {
  const ms = Number.parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue("--dur-media"),
  );
  return Number.isNaN(ms) ? 0 : ms / 1000;
}

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
    const duracion = reducir ? 0 : duracionConteo();
    if (duracion === 0 || actual.current === valor) {
      actual.current = valor;
      setMostrado(valor);
      return;
    }
    const control = animate(actual.current, valor, {
      duration: duracion,
      ease: [0.2, 0.7, 0.2, 1], // --curva-salida
      onUpdate: (v) => {
        actual.current = v;
        setMostrado(Math.round(v));
      },
    });
    return () => control.stop();
  }, [valor, reducir]);

  return <Cifra valor={mostrado} className={className} />;
}
