// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef, type KeyboardEvent } from "react";
import { useEdicion, type PanelEdicion as Panel } from "../../estado/edicion";
import { useMateriales } from "../../estado/materiales";
import { ListaParecidos } from "./ListaParecidos";
import { SeccionFamilia } from "./SeccionFamilia";
import { SeccionHistorial } from "./SeccionHistorial";
import { SeccionPendientes } from "./SeccionPendientes";
import "./Edicion.css";

const PANELES: { valor: Panel; etiqueta: string }[] = [
  { valor: "cambiar", etiqueta: "Cambiar" },
  { valor: "historial", etiqueta: "Historial" },
];

/**
 * Panel de la aside en Modificar: arriba, el bloque pinchado y sus parecidos
 * (o el historial); abajo, siempre a la vista, los pendientes con Aplicar.
 */
export function PanelEdicion() {
  const panel = useEdicion((s) => s.panel);
  const verPanel = useEdicion((s) => s.verPanel);
  const refPestanas = useRef<Record<Panel, HTMLButtonElement | null>>({
    cambiar: null,
    historial: null,
  });
  const cargarNombres = useMateriales((s) => s.cargarNombres);
  useEffect(() => cargarNombres(), [cargarNombres]);

  const alPulsarPestanas = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(e.key)) return;
    e.preventDefault();
    const destino: Panel =
      e.key === "Home"
        ? "cambiar"
        : e.key === "End"
          ? "historial"
          : panel === "cambiar"
            ? "historial"
            : "cambiar";
    verPanel(destino);
    refPestanas.current[destino]?.focus();
  };

  return (
    <div className="edicion">
      <div
        className="schematic__pestanas"
        role="tablist"
        aria-label="Edición"
        onKeyDown={alPulsarPestanas}
      >
        {PANELES.map(({ valor, etiqueta }) => (
          <button
            key={valor}
            ref={(el) => {
              refPestanas.current[valor] = el;
            }}
            type="button"
            role="tab"
            id={`pestana-edicion-${valor}`}
            className="schematic__pestana rotulo"
            aria-selected={panel === valor}
            aria-controls={panel === valor ? `panel-edicion-${valor}` : undefined}
            tabIndex={panel === valor ? 0 : -1}
            onClick={() => verPanel(valor)}
          >
            {etiqueta}
          </button>
        ))}
      </div>

      <div
        className="edicion__cuerpo"
        role="tabpanel"
        id={`panel-edicion-${panel}`}
        aria-labelledby={`pestana-edicion-${panel}`}
      >
        {panel === "cambiar" ? <SeccionCambiar /> : <SeccionHistorial />}
      </div>

      <SeccionPendientes />
    </div>
  );
}

// ── Cambiar: bloque pinchado y lista de parecidos ──

function SeccionCambiar() {
  const lista = useEdicion((s) => s.lista);
  const pinchado = useEdicion((s) => s.pinchado);
  const familia = useEdicion((s) => s.familia);

  if (familia) return <SeccionFamilia familia={familia} />;
  if (!lista && !pinchado)
    return (
      <p className="edicion__ayuda">
        Pincha un bloque del modelo para ver los más parecidos de la misma forma. Lo que elijas
        queda pendiente hasta que pulses <strong>Aplicar</strong>.
      </p>
    );
  if (!lista) return <p className="edicion__ayuda">Leyendo el bloque…</p>;
  return <ListaParecidos lista={lista} />;
}
