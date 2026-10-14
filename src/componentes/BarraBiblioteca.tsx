// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { memo, type Ref } from "react";
import { useBiblioteca, type CampoOrden, type Orden } from "../estado/biblioteca";
import { useAjustes } from "../estado/ajustes";
import { IconoAjustes, IconoBuscar, IconoLista, IconoRejilla, IconoSentido } from "./Iconos";
import "./BarraBiblioteca.css";

const CAMPOS: { campo: CampoOrden; etiqueta: string }[] = [
  { campo: "fecha", etiqueta: "Fecha" },
  { campo: "nombre", etiqueta: "Nombre" },
  { campo: "tamano", etiqueta: "Tamaño" },
];

/** «Ordenado por fecha, recientes primero»: lo que se lee y lo que se ve. */
function describirOrden({ campo, direccion }: Orden): string {
  const asc = direccion === "asc";
  switch (campo) {
    case "nombre":
      return `por nombre, ${asc ? "de la A a la Z" : "de la Z a la A"}`;
    case "tamano":
      return `por tamaño, ${asc ? "pequeños primero" : "grandes primero"}`;
    default:
      return `por fecha de importación, ${asc ? "antiguos primero" : "recientes primero"}`;
  }
}

interface Props {
  /** El campo de búsqueda, para enfocarlo con «/». */
  refBuscar: Ref<HTMLInputElement>;
  /** Esc en la búsqueda: limpia y devuelve el foco a la rejilla. */
  alSalirDeBusqueda: () => void;
}

/**
 * Barra de la biblioteca como el cajetín de un plano: casillas de la misma
 * altura separadas por filetes verticales. Buscar, ordenar, vista, importar.
 */
export const BarraBiblioteca = memo(function BarraBiblioteca({
  refBuscar,
  alSalirDeBusqueda,
}: Props) {
  const busqueda = useBiblioteca((s) => s.busqueda);
  const orden = useBiblioteca((s) => s.orden);
  const modoVista = useBiblioteca((s) => s.modoVista);
  const establecerBusqueda = useBiblioteca((s) => s.establecerBusqueda);
  const alternarOrden = useBiblioteca((s) => s.alternarOrden);
  const establecerModoVista = useBiblioteca((s) => s.establecerModoVista);
  const elegirEImportar = useBiblioteca((s) => s.elegirEImportar);
  const abrirAjustes = useAjustes((s) => s.abrir);

  return (
    // Sin role="toolbar": ese patrón pide mover el foco con las flechas, que
    // dentro del campo de búsqueda tienen que mover el cursor.
    <div className="barra-biblioteca">
      <label className="barra-biblioteca__buscar">
        <IconoBuscar />
        <span className="solo-lectores">Buscar</span>
        <input
          ref={refBuscar}
          type="search"
          value={busqueda}
          placeholder="Buscar por nombre, autor o formato"
          spellCheck={false}
          autoComplete="off"
          aria-keyshortcuts="/"
          onChange={(e) => establecerBusqueda(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              // Esc limpia y sale; preventDefault para que el Esc global no actúe.
              e.preventDefault();
              e.stopPropagation();
              establecerBusqueda("");
              alSalirDeBusqueda();
            }
          }}
        />
        {!busqueda && (
          <kbd className="barra-biblioteca__atajo" aria-hidden="true">
            /
          </kbd>
        )}
      </label>

      <div className="barra-biblioteca__grupo" role="group" aria-label="Ordenar">
        <span className="barra-biblioteca__etiqueta" aria-hidden="true">
          Ordenar
        </span>
        {CAMPOS.map(({ campo, etiqueta }) => {
          const activo = orden.campo === campo;
          return (
            <button
              key={campo}
              type="button"
              className="casilla"
              aria-pressed={activo}
              aria-label={
                activo
                  ? `${etiqueta}: ordenado ${describirOrden(orden)}. Pulsa para invertir`
                  : `Ordenar por ${etiqueta.toLowerCase()}`
              }
              title={activo ? `Ordenado ${describirOrden(orden)}` : undefined}
              onClick={() => alternarOrden(campo)}
            >
              {etiqueta}
              <span className="casilla__sentido">
                {activo && <IconoSentido ascendente={orden.direccion === "asc"} />}
              </span>
            </button>
          );
        })}
      </div>

      <div className="barra-biblioteca__grupo" role="group" aria-label="Vista">
        <button
          type="button"
          className="casilla casilla--icono"
          aria-pressed={modoVista === "rejilla"}
          aria-label="Ver como rejilla"
          title="Rejilla"
          onClick={() => establecerModoVista("rejilla")}
        >
          <IconoRejilla />
        </button>
        <button
          type="button"
          className="casilla casilla--icono"
          aria-pressed={modoVista === "lista"}
          aria-label="Ver como lista"
          title="Lista"
          onClick={() => establecerModoVista("lista")}
        >
          <IconoLista />
        </button>
      </div>

      <button
        type="button"
        className="casilla casilla--icono"
        aria-label="Ajustes"
        aria-haspopup="dialog"
        aria-keyshortcuts="Control+,"
        title="Ajustes (Ctrl+,)"
        onClick={() => void abrirAjustes()}
      >
        <IconoAjustes />
      </button>

      <button
        type="button"
        className="casilla casilla--importar"
        aria-keyshortcuts="Control+O"
        onClick={() => void elegirEImportar()}
      >
        Importar
        <kbd>Ctrl O</kbd>
      </button>
    </div>
  );
});
