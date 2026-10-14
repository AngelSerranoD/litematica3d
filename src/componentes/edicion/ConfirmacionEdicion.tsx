// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef, type KeyboardEvent } from "react";
import { m } from "motion/react";
import { formatoNumero } from "../../logica/formato";
import {
  bloquesAfectados,
  numPendientes,
  useEdicion,
  type Confirmacion,
  type MotivoDescarte,
} from "../../estado/edicion";
import { muelle } from "../../estilos/movimiento";
import "./Edicion.css";
import "./Confirmacion.css";

const PREGUNTA_DESCARTE: Record<MotivoDescarte, string> = {
  salirModo: "¿Descartarlos y volver a Visualizar?",
  salirSchematic: "¿Descartarlos y salir del schematic?",
  cerrarVentana: "¿Descartarlos y cerrar Schematic Lab?",
  restaurar: "Para restaurar otra versión hay que descartarlos. ¿Descartarlos y restaurar?",
  todo: "¿Descartar todos los cambios pendientes?",
};

const plural = (n: number, uno: string, varios: string) => (n === 1 ? uno : varios);

/**
 * Pregunta de la edición en un <dialog> modal nativo (atrapa el foco y
 * devuelve el foco al cerrarse): aplicar los pendientes, o descartarlos
 * antes de salir de Modificar, del schematic o de restaurar una versión.
 * Al descartar, el foco inicial está en el botón que no pierde nada.
 */
export function ConfirmacionEdicion() {
  const confirmacion = useEdicion((s) => s.confirmacion);
  const dialogo = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const d = dialogo.current;
    if (!d) return;
    if (confirmacion && !d.open) {
      d.showModal();
      // autoFocus de React no sirve aquí: el botón se monta con el diálogo aún cerrado.
      d.querySelector<HTMLElement>("[data-foco-inicial]")?.focus();
    } else if (!confirmacion && d.open) d.close();
  }, [confirmacion]);

  const cancelar = () => useEdicion.getState().cancelarConfirmacion();

  // Esc cierra la pregunta y nada más: preventDefault para que ni el visor ni
  // App (volver a la biblioteca) lo usen también.
  const alPulsar = (e: KeyboardEvent<HTMLDialogElement>) => {
    if (e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    cancelar();
  };

  return (
    <dialog
      ref={dialogo}
      className="confirmacion-edicion"
      aria-labelledby="titulo-confirmacion-edicion"
      aria-describedby="texto-confirmacion-edicion"
      onCancel={(e) => {
        e.preventDefault();
        cancelar();
      }}
      onKeyDown={alPulsar}
    >
      {confirmacion && <Contenido confirmacion={confirmacion} onCancelar={cancelar} />}
    </dialog>
  );
}

function Contenido({
  confirmacion,
  onCancelar,
}: {
  confirmacion: Confirmacion;
  onCancelar: () => void;
}) {
  const cambios = useEdicion((s) => s.cambios);
  const reemplazos = useEdicion((s) => s.reemplazos);
  const n = useEdicion(numPendientes);
  const confirmar = useEdicion((s) => s.confirmar);
  const { total, exacto } = bloquesAfectados(cambios, reemplazos);
  const aplicar = confirmacion.tipo === "aplicar";

  const titulo = aplicar ? "Aplicar cambios" : "Cambios sin aplicar";
  const texto = aplicar
    ? `Se ${plural(n, "aplicará 1 cambio", `aplicarán ${formatoNumero(n)} cambios`)} que ` +
      `${plural(total, "afecta", "afectan")} a ${exacto ? "" : "al menos "}` +
      `${formatoNumero(total)} ${plural(total, "bloque", "bloques")}. La versión actual queda en ` +
      "el historial para poder deshacerlo."
    : `Hay ${plural(n, "1 cambio pendiente", `${formatoNumero(n)} cambios pendientes`)}. ` +
      PREGUNTA_DESCARTE[confirmacion.motivo];

  return (
    <m.div
      className="confirmacion-edicion__caja"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={muelle("pulsacion")}
    >
      <h2 id="titulo-confirmacion-edicion" className="confirmacion-edicion__titulo rotulo">
        {titulo}
      </h2>
      <p id="texto-confirmacion-edicion" className="confirmacion-edicion__texto">
        {texto}
      </p>
      <div className="confirmacion-edicion__botones">
        {/* Al descartar, el foco empieza en «Seguir editando»: Intro no tira
            nada por descuido. Aplicar se puede deshacer: ahí empieza en Aplicar. */}
        <button
          type="button"
          className="boton-plano"
          data-foco-inicial={!aplicar || undefined}
          onClick={onCancelar}
        >
          {aplicar ? "Cancelar" : "Seguir editando"}
        </button>
        <button
          type="button"
          className={
            aplicar ? "boton-plano boton-plano--primario" : "boton-plano boton-plano--aviso"
          }
          data-foco-inicial={aplicar || undefined}
          onClick={confirmar}
        >
          {aplicar ? "Aplicar" : "Descartar"}
        </button>
      </div>
    </m.div>
  );
}
