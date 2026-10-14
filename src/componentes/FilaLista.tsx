// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { memo, useRef, type Ref } from "react";
import { m } from "motion/react";
import { type Entrada } from "../api/biblioteca";
import { muelle } from "../estilos/movimiento";
import { idBloque, idVisor } from "../estilos/idsTransicion";
import { Cifra } from "./Cifra";
import {
  salidaElemento,
  transicionElemento,
  varianteElemento,
  type AccionesElemento,
  type InfoEntrada,
} from "../logica/elementoBiblioteca";
import {
  dimensionesHabladas,
  fechaCompleta,
  fechaCorta,
  formatoCorto,
  formatoDimensiones,
  nombreFormato,
} from "../logica/formato";
import { IconoPapelera } from "./Iconos";
import { Miniatura } from "./Miniatura";
import { NombreEditable } from "./NombreEditable";

interface Props {
  entrada: Entrada;
  renombrando: boolean;
  /**
   * Lleva los layoutId del elemento compartido con la vista Schematic. Solo
   * la lámina que se abre (o de la que se vuelve): con 500 láminas, tener
   * 1000 nodos de layout más encarece cada reordenación.
   */
  compartido: boolean;
  info: InfoEntrada;
  acciones: AccionesElemento;
  ref?: Ref<HTMLTableRowElement>;
}

/**
 * Fila de la lista densa. Mismas reglas que la lámina: un clic en la fila
 * abre, salvo en el nombre (zona de texto, doble clic para renombrar) y en
 * el botón de eliminar.
 */
export const FilaLista = memo(function FilaLista({
  entrada,
  renombrando,
  compartido,
  info,
  acciones,
  ref,
}: Props) {
  const { id, nombre, dimensiones, formato, totalBloques, fechaImportacion } = entrada;
  const boton = useRef<HTMLButtonElement>(null);

  return (
    <m.tr
      ref={ref}
      className="fila"
      data-id={id}
      layout="position"
      custom={info}
      variants={varianteElemento}
      exit={salidaElemento()}
      transition={transicionElemento()}
      onClick={(e) => {
        if (renombrando) return;
        const objetivo = e.target as HTMLElement;
        if (objetivo.closest(".fila__nombre-texto, .fila__eliminar")) return;
        acciones.abrir(id);
      }}
    >
      <td className="fila__miniatura">
        <button
          ref={boton}
          type="button"
          className="fila__abrir"
          data-abrir={id}
          // Tabindex itinerante: la biblioteca pone a 0 el del elemento activo
          // directamente en el DOM, sin volver a pintar las láminas.
          tabIndex={-1}
          aria-label={nombre}
          aria-keyshortcuts="Enter F2 Delete"
          onFocus={() => acciones.activar(id)}
        >
          {/* Solo el visor compartido es un nodo de Motion: los demás, un span. */}
          {compartido ? (
            <m.span
              className="fila__visor"
              layoutId={idVisor(id)}
              transition={{ layout: muelle("compartido") }}
            >
              <Miniatura id={id} dimensiones={dimensiones} miniatura={entrada.miniatura} layoutId={idBloque(id)} />
            </m.span>
          ) : (
            <span className="fila__visor">
              <Miniatura id={id} dimensiones={dimensiones} miniatura={entrada.miniatura} />
            </span>
          )}
        </button>
      </td>
      <td className="fila__nombre">
        {renombrando ? (
          <NombreEditable
            nombre={nombre}
            className="fila__campo"
            alTerminar={(nuevo, devolver) => acciones.terminarRenombrado(id, nuevo, devolver)}
          />
        ) : (
          <span
            className="fila__nombre-texto"
            title={`${nombre}\nDoble clic o F2 para renombrar`}
            onClick={() => boton.current?.focus()}
            onDoubleClick={() => acciones.empezarRenombrado(id)}
          >
            {nombre}
          </span>
        )}
      </td>
      <td className="fila__num">
        <span className="cifras" aria-hidden="true">
          {formatoDimensiones(dimensiones, true)}
        </span>
        <span className="solo-lectores">{dimensionesHabladas(dimensiones)}</span>
      </td>
      <td className="fila__num">
        <Cifra valor={totalBloques} />
      </td>
      <td className="fila__formato" title={nombreFormato(formato)}>
        {formatoCorto(formato)}
      </td>
      <td className="fila__fecha cifras" title={fechaCompleta(fechaImportacion)}>
        {fechaCorta(fechaImportacion)}
      </td>
      <td className="fila__acciones">
        <button
          type="button"
          className="fila__eliminar boton-icono"
          tabIndex={-1}
          aria-label={`Eliminar ${nombre}`}
          title="Eliminar (Supr)"
          onClick={(e) => {
            e.stopPropagation();
            acciones.eliminar(id);
          }}
        >
          <IconoPapelera />
        </button>
      </td>
    </m.tr>
  );
});
