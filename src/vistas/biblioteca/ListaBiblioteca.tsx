// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// La colección de la biblioteca como tabla ordenable.

import { AnimatePresence } from "motion/react";
import { FilaLista } from "../../componentes/FilaLista";
import { IconoSentido } from "../../componentes/Iconos";
import { useBiblioteca, type CampoOrden, type Orden } from "../../estado/biblioteca";
import type { PropsColeccion } from "./RejillaBiblioteca";

export function ListaBiblioteca({
  refContenedor,
  visibles,
  renombrandoId,
  compartidoId,
  infoEntrada,
  acciones,
  idAyuda,
}: PropsColeccion) {
  const orden = useBiblioteca((s) => s.orden);
  const alternarOrden = useBiblioteca((s) => s.alternarOrden);
  return (
    <table
      ref={refContenedor}
      className="lista"
      aria-label={`Schematics: ${visibles.length}`}
      aria-describedby={idAyuda}
    >
      <thead>
        <tr>
          <th scope="col" className="lista__miniatura">
            <span className="solo-lectores">Miniatura</span>
          </th>
          <CabeceraOrdenable
            campo="nombre"
            etiqueta="Nombre"
            orden={orden}
            alPulsar={alternarOrden}
          />
          <th scope="col" className="lista__num">
            Medidas
          </th>
          <CabeceraOrdenable
            campo="tamano"
            etiqueta="Bloques"
            orden={orden}
            alPulsar={alternarOrden}
            numerica
          />
          <th scope="col">Formato</th>
          <CabeceraOrdenable
            campo="fecha"
            etiqueta="Importado"
            orden={orden}
            alPulsar={alternarOrden}
          />
          <th scope="col" className="lista__acciones">
            <span className="solo-lectores">Eliminar</span>
          </th>
        </tr>
      </thead>
      <tbody>
        <AnimatePresence>
          {visibles.map((e, i) => (
            <FilaLista
              key={e.id}
              entrada={e}
              renombrando={e.id === renombrandoId}
              compartido={e.id === compartidoId}
              info={infoEntrada(e.id, i)}
              acciones={acciones}
            />
          ))}
        </AnimatePresence>
      </tbody>
    </table>
  );
}

interface PropsCabecera {
  campo: CampoOrden;
  etiqueta: string;
  orden: Orden;
  alPulsar: (campo: CampoOrden) => void;
  numerica?: boolean;
}

/** Cabecera de columna que ordena; el clic en la activa invierte el sentido. */
function CabeceraOrdenable({ campo, etiqueta, orden, alPulsar, numerica }: PropsCabecera) {
  const activa = orden.campo === campo;
  const ascendente = orden.direccion === "asc";
  return (
    <th
      scope="col"
      className={numerica ? "lista__num" : undefined}
      aria-sort={activa ? (ascendente ? "ascending" : "descending") : "none"}
    >
      <button type="button" className="lista__ordenar" onClick={() => alPulsar(campo)}>
        {etiqueta}
        <span className="casilla__sentido">
          {activa && <IconoSentido ascendente={ascendente} />}
        </span>
      </button>
    </th>
  );
}
