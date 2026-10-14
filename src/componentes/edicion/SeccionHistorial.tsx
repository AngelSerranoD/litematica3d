// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Historial de versiones del schematic, con Restaurar.

import { useEffect } from "react";
import { MAX_VERSIONES } from "../../api/edicion";
import { useEdicion } from "../../estado/edicion";
import { Cifra } from "../Cifra";
import { fechaCompleta, nombreFormato } from "../../logica/formato";
import "./Historial.css";

export function SeccionHistorial() {
  const versiones = useEdicion((s) => s.versiones);
  const cargando = useEdicion((s) => s.cargandoHistorial);
  const error = useEdicion((s) => s.errorHistorial);
  const ocupado = useEdicion((s) => s.ocupado);
  const restaurar = useEdicion((s) => s.restaurar);

  // Al montar (p. ej. al volver a la pestaña) se pide otra vez: puede haber cambiado.
  useEffect(() => {
    void useEdicion.getState().cargarHistorial();
  }, []);

  if (error)
    return (
      <p className="edicion__error" role="alert">
        No se pudo leer el historial: {error}
      </p>
    );
  if (!versiones)
    return (
      <p className="edicion__ayuda" role="status">
        {cargando ? "Leyendo el historial…" : ""}
      </p>
    );
  if (versiones.length === 0)
    return (
      <p className="edicion__ayuda">
        Aún no hay versiones anteriores. Cada vez que apliques cambios, la versión de antes se
        guarda aquí (las {MAX_VERSIONES} últimas) y se puede restaurar.
      </p>
    );

  return (
    <ul className="historial" aria-label="Versiones anteriores" aria-busy={cargando}>
      {versiones.map((v) => (
        <li key={v.version} className="historial__fila">
          <div className="historial__datos">
            <p className="historial__fecha">{fechaCompleta(v.fecha)}</p>
            <p className="historial__detalle">
              {v.motivo === "aplicar" ? (
                <>
                  Antes de cambiar <Cifra valor={v.cambios} /> {v.cambios === 1 ? "bloque" : "bloques"}
                </>
              ) : (
                "Antes de restaurar otra versión"
              )}
            </p>
            <p className="historial__detalle">
              {nombreFormato(v.formato)} · <Cifra valor={v.totalBloques} /> bloques
            </p>
          </div>
          <button
            type="button"
            className="boton-plano"
            disabled={ocupado !== null}
            aria-label={`Restaurar la versión del ${fechaCompleta(v.fecha)}`}
            onClick={() => void restaurar(v.version)}
          >
            Restaurar
          </button>
        </li>
      ))}
    </ul>
  );
}
