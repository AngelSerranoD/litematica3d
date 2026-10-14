// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Pendientes de la edición, siempre a la vista, con Descartar y Aplicar.

import { AnimatePresence, m } from "motion/react";
import {
  bloquesAfectados,
  numPendientes,
  useEdicion,
  type CambioPendiente,
  type ReemplazoPendiente,
} from "../../estado/edicion";
import { muelle } from "../../estilos/movimiento";
import { CifraAnimada } from "../CifraAnimada";
import { formatoNumero } from "../../logica/formato";
import { Bloque, useNombre } from "./BloqueEdicion";
import "./Parecidos.css";
import "./Pendientes.css";

export function SeccionPendientes() {
  const cambios = useEdicion((s) => s.cambios);
  const reemplazos = useEdicion((s) => s.reemplazos);
  const n = useEdicion(numPendientes);
  const ocupado = useEdicion((s) => s.ocupado);
  const error = useEdicion((s) => s.error);
  const { total, exacto } = bloquesAfectados(cambios, reemplazos);

  return (
    <section className="pendientes-edicion" aria-labelledby="titulo-pendientes-edicion">
      <header className="pendientes-edicion__cabecera">
        <h2 id="titulo-pendientes-edicion" className="rotulo pendientes-edicion__titulo">
          Pendientes
        </h2>
        <CifraAnimada valor={n} className="pendientes-edicion__cuenta" />
      </header>

      {n === 0 ? (
        <p className="edicion__ayuda">Nada pendiente.</p>
      ) : (
        <ul className="pendientes-edicion__lista" aria-label="Cambios pendientes">
          <AnimatePresence initial={false}>
            {reemplazos.map((r) => (
              <FilaReemplazo key={`r:${r.estadoOrigen}`} reemplazo={r} />
            ))}
            {cambios.map((c) => (
              <FilaCambio key={`c:${c.clave}`} cambio={c} />
            ))}
          </AnimatePresence>
        </ul>
      )}

      {error && (
        <p className="edicion__error" role="alert">
          {error}
        </p>
      )}

      <div className="pendientes-edicion__acciones">
        <span className="pendientes-edicion__total">
          {n > 0 && (
            <>
              {exacto ? "" : "≥ "}
              <span className="cifras">{formatoNumero(total)}</span>{" "}
              {total === 1 ? "bloque" : "bloques"}
            </>
          )}
        </span>
        <button
          type="button"
          className="boton-plano"
          disabled={n === 0 || ocupado !== null}
          onClick={() => useEdicion.getState().pedirDescartar("todo", () => {})}
        >
          Descartar todo
        </button>
        <button
          type="button"
          className="boton-plano boton-plano--primario"
          disabled={n === 0 || ocupado !== null}
          aria-busy={ocupado === "aplicar"}
          onClick={() => useEdicion.getState().pedirAplicar()}
        >
          {ocupado === "aplicar" ? "Aplicando…" : "Aplicar…"}
        </button>
      </div>
    </section>
  );
}

/** Fila de la lista de pendientes: origen → destino, dónde, y quitar. */
function FilaPendiente({
  origen,
  destino,
  donde,
  onQuitar,
}: {
  origen: string;
  destino: string;
  donde: string;
  onQuitar: () => void;
}) {
  const nombreOrigen = useNombre(origen);
  const nombreDestino = useNombre(destino);
  return (
    <m.li
      className="pendiente-edicion"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={muelle("pulsacion")}
    >
      <span className="pendiente-edicion__bloques">
        <Bloque estado={origen} />
        <span className="pendiente-edicion__flecha" aria-hidden="true">
          →
        </span>
        <span className="edicion__solo-lectores">por</span>
        <Bloque estado={destino} />
      </span>
      <span className="pendiente-edicion__donde cifras">{donde}</span>
      <button
        type="button"
        className="pendiente-edicion__quitar"
        aria-label={`Quitar: ${nombreOrigen} por ${nombreDestino}, ${donde}`}
        title="Quitar"
        onClick={onQuitar}
      >
        <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
          <path d="M1 1L9 9M9 1L1 9" />
        </svg>
      </button>
    </m.li>
  );
}

function FilaCambio({ cambio }: { cambio: CambioPendiente }) {
  const quitar = useEdicion((s) => s.quitarCambio);
  return (
    <FilaPendiente
      origen={cambio.estadoAnterior}
      destino={cambio.nuevoEstado}
      donde={`${cambio.x}, ${cambio.y}, ${cambio.z}`}
      onQuitar={() => quitar(cambio.clave)}
    />
  );
}

function FilaReemplazo({ reemplazo }: { reemplazo: ReemplazoPendiente }) {
  const quitar = useEdicion((s) => s.quitarReemplazo);
  const donde =
    reemplazo.cantidad === null
      ? "todos"
      : `${formatoNumero(reemplazo.cantidad)} ${reemplazo.cantidad === 1 ? "bloque" : "bloques"}`;
  return (
    <FilaPendiente
      origen={reemplazo.estadoOrigen}
      destino={reemplazo.estadoDestino}
      donde={donde}
      onQuitar={() => quitar(reemplazo.estadoOrigen)}
    />
  );
}
