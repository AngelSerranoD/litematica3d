// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Tabla origen → destino de la familia seleccionada.

import { useMemo } from "react";
import { useEdicion, type ListaParecidos as Lista, type SeleccionFamilia } from "../../estado/edicion";
import { agruparMiembros, type GrupoFamilia } from "../../estado/edicionPura";
import { formatoNumero } from "../../logica/formato";
import { Bloque, useNombre } from "./BloqueEdicion";
import { ListaParecidos } from "./ListaParecidos";
import "./Parecidos.css";

/**
 * Tabla origen → destino de la familia: una fila por bloque presente (los
 * estados con distinto `facing`, `half`… se agrupan; cada uno conserva sus
 * propiedades al aplicar).
 */
export function TablaFamilia({ familia }: { familia: SeleccionFamilia }) {
  const reemplazos = useEdicion((s) => s.reemplazos);
  const lista = useEdicion((s) => s.lista);
  const destinoDe = new Map(reemplazos.map((r) => [r.estadoOrigen, r.estadoDestino]));
  const abierta = lista?.objetivo.tipo === "reemplazo" ? lista.objetivo.estadoOrigen : null;
  const grupos = useMemo(() => agruparMiembros(familia.miembros), [familia.miembros]);
  const destinoGrupo = (g: GrupoFamilia) =>
    g.estados.map((e) => destinoDe.get(e)).find((d) => d !== undefined) ?? null;
  const cambian = grupos.filter((g) => destinoGrupo(g) !== null).length;

  return (
    <>
      <p className="parecidos__titulo">
        <span className="rotulo">Origen → destino</span>
        <span className="parecidos__leyenda cifras">
          {formatoNumero(cambian)} de {formatoNumero(grupos.length)} cambian
        </span>
      </p>
      <ul className="tabla-familia" aria-label="Bloques de la familia y por qué se cambian">
        {grupos.map((g) => (
          <FilaFamilia
            key={g.id}
            estados={g.estados}
            cantidad={g.cantidad}
            destino={destinoGrupo(g)}
            lista={abierta === g.estados[0] && lista ? lista : null}
          />
        ))}
      </ul>
    </>
  );
}

function FilaFamilia({
  estados,
  cantidad,
  destino,
  lista,
}: {
  /** Estados del mismo bloque (el primero lo representa). */
  estados: string[];
  cantidad: number;
  destino: string | null;
  /** Lista de parecidos abierta para esta fila. */
  lista: Lista | null;
}) {
  const estado = estados[0];
  const nombre = useNombre(estado);
  const nombreDestino = useNombre(destino ?? estado);
  const idLista = `lista-familia-${estado.replace(/[^a-z0-9]+/gi, "-")}`;
  const alternar = () => {
    const e = useEdicion.getState();
    if (lista) e.cerrarLista();
    else e.abrirLista({ tipo: "reemplazo", estadoOrigen: estado, hermanos: estados.slice(1) });
  };
  return (
    <li className="fila-familia">
      <div className="pendiente-edicion">
        <span className="pendiente-edicion__bloques">
          <Bloque estado={estado} />
          <span className="pendiente-edicion__flecha" aria-hidden="true">
            →
          </span>
          {destino ? (
            <>
              <span className="edicion__solo-lectores">por</span>
              <Bloque estado={destino} />
            </>
          ) : (
            <span className="fila-familia__igual">se queda igual</span>
          )}
        </span>
        <span className="pendiente-edicion__donde cifras">{formatoNumero(cantidad)}</span>
        <button
          type="button"
          className="boton-plano boton-plano--mini"
          aria-expanded={lista !== null}
          aria-controls={lista ? idLista : undefined}
          aria-label={`Elegir por qué cambiar ${nombre}`}
          onClick={alternar}
        >
          Cambiar…
        </button>
        {destino && (
          <button
            type="button"
            className="pendiente-edicion__quitar"
            aria-label={`Dejar ${nombre} como está (no cambiarlo por ${nombreDestino})`}
            title="Dejar como está"
            onClick={() => estados.forEach((e) => useEdicion.getState().quitarReemplazo(e))}
          >
            <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
              <path d="M1 1L9 9M9 1L1 9" />
            </svg>
          </button>
        )}
      </div>
      {lista && (
        <div id={idLista} className="fila-familia__lista">
          <ListaParecidos lista={lista} />
        </div>
      )}
    </li>
  );
}
