// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef } from "react";
import { hayCambios, useAjustes, type EstadoAtlas } from "../estado/ajustes";
import "./PanelAjustes.css";

/** Texto del estado del atlas para la versión elegida. */
function describirAtlas(atlas: EstadoAtlas, hayVersiones: boolean): string {
  if (!hayVersiones || atlas.tipo === "sinJar") return "Sin texturas: se usan colores planos.";
  switch (atlas.tipo) {
    case "listo":
      return `Texturas listas (Minecraft ${atlas.datos.version}, atlas de ${atlas.datos.ancho} × ${atlas.datos.alto} px).`;
    case "preparando":
      return "Preparando las texturas…";
    default:
      return "Comprobando las texturas…";
  }
}

/**
 * Ajustes de la app en un diálogo modal: de momento, la versión de Minecraft
 * de la que salen las texturas del visor 3D. Usa <dialog> nativo, que ya
 * atrapa el foco, inhabilita el resto de la página y se cierra con Esc.
 */
export function PanelAjustes() {
  const abierto = useAjustes((s) => s.abierto);
  const cargando = useAjustes((s) => s.cargando);
  const guardando = useAjustes((s) => s.guardando);
  const error = useAjustes((s) => s.error);
  const versiones = useAjustes((s) => s.versiones);
  const elegida = useAjustes((s) => s.elegida);
  const atlas = useAjustes((s) => s.atlas);
  const cambios = useAjustes(hayCambios);
  const elegir = useAjustes((s) => s.elegir);
  const guardar = useAjustes((s) => s.guardar);
  const cerrar = useAjustes((s) => s.cerrar);
  const dialogo = useRef<HTMLDialogElement>(null);

  // El estado manda: el <dialog> se abre y se cierra para seguirlo.
  useEffect(() => {
    const d = dialogo.current;
    if (!d) return;
    if (abierto && !d.open) d.showModal();
    else if (!abierto && d.open) d.close();
  }, [abierto]);

  // Releases primero (el orden ya viene así de Rust) y las demás detrás.
  const releases = versiones.filter((v) => v.esRelease);
  const otras = versiones.filter((v) => !v.esRelease);
  const sinVersiones = !cargando && versiones.length === 0;

  return (
    <dialog
      ref={dialogo}
      className="ajustes"
      aria-labelledby="titulo-ajustes"
      // Esc: se cierra por el estado, para que no se desfase del <dialog>.
      onCancel={(e) => {
        e.preventDefault();
        cerrar();
      }}
      onClick={(e) => {
        // Clic en el fondo (fuera de la caja).
        if (e.target === e.currentTarget) cerrar();
      }}
    >
      <form
        className="ajustes__caja"
        onSubmit={(e) => {
          e.preventDefault();
          void guardar();
        }}
      >
        <h2 id="titulo-ajustes" className="ajustes__titulo rotulo">
          Ajustes
        </h2>

        <div className="ajustes__campo">
          <label htmlFor="ajustes-version" className="ajustes__etiqueta">
            Versión de Minecraft para las texturas
          </label>
          <select
            id="ajustes-version"
            className="ajustes__selector"
            value={elegida ?? ""}
            disabled={cargando || sinVersiones || guardando}
            onChange={(e) => elegir(e.target.value)}
          >
            {sinVersiones && <option value="">Ninguna versión instalada</option>}
            {releases.length > 0 && (
              <optgroup label="Versiones oficiales">
                {releases.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.id}
                  </option>
                ))}
              </optgroup>
            )}
            {otras.length > 0 && (
              <optgroup label="Otras (snapshots, Fabric, Forge…)">
                {otras.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.id} (no es release)
                  </option>
                ))}
              </optgroup>
            )}
          </select>
          <p className="ajustes__ayuda">
            Las texturas se leen del jar de tu Minecraft; la app no las redistribuye.
          </p>
        </div>

        <p className="ajustes__estado" role="status" aria-live="polite">
          {describirAtlas(atlas, versiones.length > 0)}
        </p>

        {error && (
          <p className="ajustes__error" role="alert">
            {error}
          </p>
        )}

        <div className="ajustes__acciones">
          <button type="button" className="boton-plano" onClick={cerrar}>
            Cerrar
          </button>
          <button
            type="submit"
            className="boton-plano ajustes__guardar"
            disabled={!cambios || guardando}
          >
            {guardando ? "Guardando…" : "Guardar"}
          </button>
        </div>
      </form>
    </dialog>
  );
}
