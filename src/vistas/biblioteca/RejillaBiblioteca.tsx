// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// La colección de la biblioteca como rejilla de láminas.

import type { RefObject } from "react";
import { AnimatePresence } from "motion/react";
import type { Entrada } from "../../api/biblioteca";
import type { AccionesElemento, InfoEntrada } from "../../logica/elementoBiblioteca";
import { Lamina } from "../../componentes/Lamina";

/** Lo común a la rejilla y la lista de la biblioteca. */
export interface PropsColeccion {
  refContenedor: RefObject<HTMLUListElement & HTMLTableElement | null>;
  visibles: Entrada[];
  renombrandoId: string | null;
  /** Elemento con layoutId compartido con la vista Schematic (ver Lamina). */
  compartidoId: string | null;
  infoEntrada: (id: string, indice: number) => InfoEntrada;
  acciones: AccionesElemento;
  /** Id del párrafo de ayuda del teclado. */
  idAyuda: string;
}

export function RejillaBiblioteca({
  refContenedor,
  visibles,
  renombrandoId,
  compartidoId,
  infoEntrada,
  acciones,
  idAyuda,
}: PropsColeccion) {
  return (
    <ul
      ref={refContenedor}
      className="rejilla"
      aria-label={`Schematics: ${visibles.length}`}
      aria-describedby={idAyuda}
    >
      <AnimatePresence mode="popLayout">
        {visibles.map((e, i) => (
          <Lamina
            key={e.id}
            entrada={e}
            renombrando={e.id === renombrandoId}
            compartido={e.id === compartidoId}
            info={infoEntrada(e.id, i)}
            acciones={acciones}
          />
        ))}
      </AnimatePresence>
    </ul>
  );
}
