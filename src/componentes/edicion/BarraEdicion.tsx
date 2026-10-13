// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { m } from "motion/react";
import { useBiblioteca } from "../../estado/biblioteca";
import { useEdicion, type ModoVisor } from "../../estado/edicion";
import { muelle } from "../../estilos/movimiento";
import "./Edicion.css";

const MODOS: { valor: ModoVisor; etiqueta: string }[] = [
  { valor: "visualizar", etiqueta: "Visualizar" },
  { valor: "modificar", etiqueta: "Modificar" },
];

/**
 * Conmutador Visualizar | Modificar de la barra del visor (atajo M). Es un
 * grupo de radios nativos: las flechas cambian de modo y el lector de
 * pantalla lo anuncia como «grupo de opciones».
 */
export function ConmutadorModo() {
  const modo = useEdicion((s) => s.modo);
  const fijarModo = useEdicion((s) => s.fijarModo);

  return (
    <div className="modo" role="radiogroup" aria-label="Modo del visor (M)">
      {MODOS.map((opcion) => (
        <label key={opcion.valor} className="modo__opcion">
          <input
            type="radio"
            name="modo-visor"
            className="modo__entrada"
            value={opcion.valor}
            checked={modo === opcion.valor}
            aria-keyshortcuts="M"
            onChange={() => fijarModo(opcion.valor)}
          />
          {modo === opcion.valor && (
            <m.span
              layoutId="modo-visor-marca"
              className="modo__marca"
              transition={muelle("pulsacion")}
              aria-hidden="true"
            />
          )}
          <span className="modo__texto">{opcion.etiqueta}</span>
        </label>
      ))}
      <kbd className="modo__tecla" aria-hidden="true">
        M
      </kbd>
    </div>
  );
}

/** Exporta la copia de la biblioteca como .litematic (diálogo de guardado). */
export function BotonExportar({ id }: { id: string }) {
  const ocupado = useEdicion((s) => s.ocupado);
  return (
    <button
      type="button"
      className="boton-plano"
      disabled={ocupado !== null}
      aria-busy={ocupado === "exportar"}
      title="Guardar la copia actual como .litematic (los pendientes no van incluidos)"
      onClick={() => {
        const entrada = useBiblioteca.getState().entradas.find((x) => x.id === id);
        void useEdicion.getState().exportar(entrada?.nombre ?? "schematic");
      }}
    >
      {ocupado === "exportar" ? "Exportando…" : "Exportar…"}
    </button>
  );
}
