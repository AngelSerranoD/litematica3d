// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useVisor } from "../../estado/visor";

/** Ficha del bloque seleccionado con el ratón: nombre, posición y propiedades. */
export function PanelBloque() {
  const seleccion = useVisor((s) => s.seleccion);
  if (!seleccion) return null;
  const { bloque } = seleccion;
  const propiedades = bloque ? Object.entries(bloque.propiedades) : [];

  return (
    <section className="panel-bloque" aria-label="Bloque seleccionado" aria-live="polite">
      <header className="panel-bloque__cabecera">
        <h2 className="panel-bloque__nombre cifras">{bloque?.nombre ?? "…"}</h2>
        <button
          type="button"
          className="panel-bloque__cerrar"
          aria-label="Cerrar (Esc)"
          title="Cerrar (Esc)"
          onClick={() => useVisor.getState().limpiarSeleccion()}
        >
          <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
            <path d="M1 1L9 9M9 1L1 9" />
          </svg>
        </button>
      </header>
      {bloque && (
        <table className="panel-bloque__tabla">
          <tbody>
            <tr>
              <th scope="row">Posición</th>
              <td className="cifras">{bloque.posicion.join(", ")}</td>
            </tr>
            {bloque.region && (
              <tr>
                <th scope="row">Región</th>
                <td>{bloque.region}</td>
              </tr>
            )}
            {propiedades.map(([clave, valor]) => (
              <tr key={clave}>
                <th scope="row" className="cifras">
                  {clave}
                </th>
                <td className="cifras">{valor}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
