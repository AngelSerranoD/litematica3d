// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useMemo, useState, type CSSProperties } from "react";
import { AnimatePresence, m } from "motion/react";
import { ESPERA_BORRADO_MS, useBiblioteca, type PendienteDeBorrar } from "../estado/biblioteca";
import { useFoco } from "../estado/foco";
import { muelle } from "../estilos/movimiento";
import "./Pendientes.css";

/** El borrado pendiente más reciente: el que deshace Ctrl+Z. */
export function ultimoPendiente(
  pendientes: Record<string, PendienteDeBorrar>,
): PendienteDeBorrar | undefined {
  let ultimo: PendienteDeBorrar | undefined;
  for (const p of Object.values(pendientes)) {
    if (!ultimo || p.vence > ultimo.vence) ultimo = p;
  }
  return ultimo;
}

/**
 * Avisos de «va a la Papelera» con Deshacer. Cada uno lleva una cuenta
 * atrás: el filete superior es una barra que se encoge hasta `vence`.
 */
export function Pendientes() {
  const pendientes = useBiblioteca((s) => s.pendientesDeBorrar);
  const lista = useMemo(
    () => Object.values(pendientes).sort((a, b) => a.vence - b.vence),
    [pendientes],
  );
  const ultimo = lista[lista.length - 1]?.entrada.id;

  return (
    <div className="pendientes" role="status" aria-live="polite">
      <AnimatePresence initial={false}>
        {lista.map((p) => (
          <FilaPendiente key={p.entrada.id} pendiente={p} conAtajo={p.entrada.id === ultimo} />
        ))}
      </AnimatePresence>
    </div>
  );
}

function FilaPendiente({
  pendiente,
  conAtajo,
}: {
  pendiente: PendienteDeBorrar;
  conAtajo: boolean;
}) {
  const deshacer = useBiblioteca((s) => s.deshacerEliminar);
  const pedirFoco = useFoco((s) => s.pedir);
  const { entrada, vence } = pendiente;
  const segundos = useSegundosHasta(vence);

  // La barra arranca donde toque (si la vista se monta con el borrado ya en
  // marcha) y llega a cero justo en `vence`. Se calcula una sola vez.
  const [estiloBarra] = useState(() => {
    const restante = Math.max(0, vence - Date.now());
    return {
      "--inicio": String(restante / ESPERA_BORRADO_MS),
      animationDuration: `${restante}ms`,
    } as CSSProperties;
  });

  return (
    <m.div
      className="pendiente"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={muelle("pulsacion")}
    >
      <span className="pendiente__cuenta" style={estiloBarra} aria-hidden="true" />
      <p className="pendiente__texto">
        «<span className="pendiente__nombre">{entrada.nombre}</span>» irá a la Papelera
      </p>
      <span className="pendiente__segundos cifras" aria-hidden="true">
        {segundos} s
      </span>
      <button
        type="button"
        className="boton-plano"
        aria-keyshortcuts={conAtajo ? "Control+Z" : undefined}
        onClick={() => {
          deshacer(entrada.id);
          pedirFoco(entrada.id);
        }}
      >
        Deshacer
        {conAtajo && <kbd>Ctrl Z</kbd>}
      </button>
    </m.div>
  );
}

const segundosHasta = (vence: number) => Math.max(0, Math.ceil((vence - Date.now()) / 1000));

/** Segundos que faltan hasta `vence`, redondeados hacia arriba. */
function useSegundosHasta(vence: number): number {
  const [segundos, setSegundos] = useState(() => segundosHasta(vence));
  useEffect(() => {
    const t = window.setInterval(() => {
      const quedan = segundosHasta(vence);
      setSegundos(quedan);
      // A cero ya no cambia: no hace falta seguir mirando el reloj.
      if (quedan === 0) window.clearInterval(t);
    }, 200);
    return () => window.clearInterval(t);
  }, [vence]);
  return segundos;
}
