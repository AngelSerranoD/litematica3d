// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import type { Region } from "../../api/biblioteca";
import type { CampoOrdenMateriales, OrdenMateriales } from "../../logica/materiales";

const ORDENES: { campo: CampoOrdenMateriales; etiqueta: string }[] = [
  { campo: "cantidad", etiqueta: "Cantidad" },
  { campo: "nombre", etiqueta: "Nombre" },
];

interface Props {
  busqueda: string;
  setBusqueda: (v: string) => void;
  orden: OrdenMateriales;
  alternarOrden: (campo: CampoOrdenMateriales) => void;
  ocultarReunidos: boolean;
  setOcultarReunidos: (v: boolean) => void;
  regiones: Region[];
  region: string | null;
  setRegion: (v: string | null) => void;
  soloVisibles: boolean;
  setSoloVisibles: (v: boolean) => void;
  /** «Y 5–44», o `null` si aún no hay rango. */
  rotuloCapas: string | null;
}

/** Barra de filtros de materiales: búsqueda, orden, reunidos, región y capas. */
export function BarraFiltros({
  busqueda,
  setBusqueda,
  orden,
  alternarOrden,
  ocultarReunidos,
  setOcultarReunidos,
  regiones,
  region,
  setRegion,
  soloVisibles,
  setSoloVisibles,
  rotuloCapas,
}: Props) {
  return (
    <div className="materiales__filtros">
      <label className="materiales__buscar">
        <span className="solo-lectores">Buscar material</span>
        <input
          type="search"
          value={busqueda}
          placeholder="Buscar por nombre o ID"
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => setBusqueda(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape" && busqueda) {
              // Esc limpia la búsqueda; preventDefault para no volver a la biblioteca.
              e.preventDefault();
              setBusqueda("");
            }
          }}
        />
      </label>

      <div className="materiales__fila-filtros">
        <div className="materiales__grupo" role="group" aria-label="Ordenar">
          <span className="materiales__etiqueta rotulo" aria-hidden="true">
            Orden
          </span>
          {ORDENES.map(({ campo, etiqueta }) => {
            const activo = orden.campo === campo;
            const sentido = orden.ascendente ? "ascendente" : "descendente";
            return (
              <button
                key={campo}
                type="button"
                className="materiales__casilla"
                aria-pressed={activo}
                aria-label={
                  activo
                    ? `${etiqueta}: orden ${sentido}. Pulsa para invertir`
                    : `Ordenar por ${etiqueta.toLowerCase()}`
                }
                onClick={() => alternarOrden(campo)}
              >
                {etiqueta}
                <span className="materiales__sentido" aria-hidden="true">
                  {activo && (orden.ascendente ? "↑" : "↓")}
                </span>
              </button>
            );
          })}
        </div>
        <label className="materiales__interruptor">
          <input
            type="checkbox"
            className="casilla-verificacion"
            checked={ocultarReunidos}
            onChange={(e) => setOcultarReunidos(e.target.checked)}
          />
          Ocultar reunidos
        </label>
      </div>

      <div className="materiales__fila-filtros">
        {regiones.length > 1 && (
          <label className="materiales__region">
            <span className="materiales__etiqueta rotulo">Región</span>
            <select
              value={region ?? ""}
              onChange={(e) => setRegion(e.target.value === "" ? null : e.target.value)}
            >
              <option value="">Todas</option>
              {regiones.map((r) => (
                <option key={r.nombre} value={r.nombre}>
                  {r.nombre}
                </option>
              ))}
            </select>
          </label>
        )}
        <label className="materiales__interruptor">
          <input
            type="checkbox"
            className="casilla-verificacion"
            checked={soloVisibles}
            onChange={(e) => setSoloVisibles(e.target.checked)}
          />
          Solo capas visibles
          <span className="materiales__capas cifras" aria-live="polite">
            {rotuloCapas ?? (soloVisibles ? "…" : "todas")}
          </span>
        </label>
      </div>
    </div>
  );
}
