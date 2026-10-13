// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";
import { useBiblioteca } from "../estado/biblioteca";
import { muelle } from "../estilos/movimiento";
import { Pendientes } from "./Pendientes";
import "./Avisos.css";

/** Superposición de «soltar archivos»: filete discontinuo y un texto. */
export function ZonaSoltar({ visible }: { visible: boolean }) {
  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          className="zona-soltar"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={muelle("pulsacion")}
        >
          <p className="zona-soltar__texto rotulo">Suelta para importar</p>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

/** Franja plana inferior: borrados por deshacer y avisos (cada aviso caduca solo). */
export function Avisos() {
  const avisos = useBiblioteca((s) => s.avisos);
  const descartar = useBiblioteca((s) => s.descartarAviso);

  // Un temporizador por aviso, ajustado a su fecha de caducidad.
  useEffect(() => {
    const temporizadores = avisos.map((a) =>
      window.setTimeout(() => descartar(a.id), Math.max(0, a.caduca - Date.now())),
    );
    return () => temporizadores.forEach((t) => window.clearTimeout(t));
  }, [avisos, descartar]);

  return (
    <div className="avisos">
      <Pendientes />
      <div className="avisos__lista" role="status" aria-live="polite">
        <AnimatePresence initial={false}>
          {avisos.map((a) => (
            <motion.div
              key={a.id}
              className="aviso"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={muelle("pulsacion")}
            >
              <p className="aviso__texto">{a.mensaje}</p>
              <button type="button" className="boton-plano" onClick={() => descartar(a.id)}>
                Cerrar
              </button>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}
