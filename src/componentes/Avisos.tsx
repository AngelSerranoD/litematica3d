// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { AnimatePresence, m } from "motion/react";
import { useBiblioteca } from "../estado/biblioteca";
import { muelle } from "../estilos/movimiento";
import { Pendientes } from "./Pendientes";
import "./Avisos.css";

/** Superposición de «soltar archivos»: filete discontinuo y un texto. */
export function ZonaSoltar({ visible }: { visible: boolean }) {
  return (
    <AnimatePresence>
      {visible && (
        <m.div
          className="zona-soltar"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={muelle("pulsacion")}
        >
          <p className="zona-soltar__texto rotulo">Suelta para importar</p>
        </m.div>
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
            <m.div
              key={a.id}
              className="aviso"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={muelle("pulsacion")}
            >
              <p className="aviso__texto">{a.mensaje}</p>
              {a.accion && (
                <button
                  type="button"
                  className="boton-plano"
                  onClick={() => {
                    descartar(a.id);
                    a.accion?.ejecutar();
                  }}
                >
                  {a.accion.etiqueta}
                </button>
              )}
              <button type="button" className="boton-plano" onClick={() => descartar(a.id)}>
                Cerrar
              </button>
            </m.div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}
