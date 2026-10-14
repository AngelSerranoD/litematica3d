// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { memo, useId, useRef, type Ref } from "react";
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
import { dimensionesHabladas, formatoCorto, formatoDimensiones, nombreFormato } from "../logica/formato";
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
  /** AnimatePresence en modo popLayout necesita la referencia del <li>. */
  ref?: Ref<HTMLLIElement>;
}

/**
 * Celda de la hoja de contactos. El visor abre el schematic; el nombre es
 * zona de texto (doble clic para renombrar) y no abre, para que el primer
 * clic de un doble clic no se lleve al usuario a otra vista.
 */
export const Lamina = memo(function Lamina({
  entrada,
  renombrando,
  compartido,
  info,
  acciones,
  ref,
}: Props) {
  const { id, nombre, dimensiones, formato, totalBloques } = entrada;
  const idMeta = useId();
  const boton = useRef<HTMLButtonElement>(null);

  return (
    <m.li
      ref={ref}
      className="lamina"
      data-id={id}
      layout
      custom={info}
      variants={varianteElemento}
      exit={salidaElemento()}
      transition={transicionElemento()}
      onClick={(e) => {
        if (renombrando) return;
        const objetivo = e.target as HTMLElement;
        if (objetivo.closest(".lamina__nombre, .lamina__eliminar")) return;
        acciones.abrir(id);
      }}
    >
      <button
        ref={boton}
        type="button"
        className="lamina__abrir"
        data-abrir={id}
        // Tabindex itinerante: la biblioteca pone a 0 el del elemento activo
        // directamente en el DOM, sin volver a pintar las láminas.
        tabIndex={-1}
        aria-label={nombre}
        aria-describedby={idMeta}
        aria-keyshortcuts="Enter F2 Delete"
        onFocus={() => acciones.activar(id)}
      >
        <span className="lamina__formato" aria-hidden="true" title={nombreFormato(formato)}>
          {formatoCorto(formato)}
        </span>
        {/* Solo el visor compartido es un nodo de Motion: los demás, un span. */}
        {compartido ? (
          <m.span
            className="lamina__visor"
            layoutId={idVisor(id)}
            transition={{ layout: muelle("compartido") }}
          >
            <Miniatura id={id} dimensiones={dimensiones} miniatura={entrada.miniatura} layoutId={idBloque(id)} />
          </m.span>
        ) : (
          <span className="lamina__visor">
            <Miniatura id={id} dimensiones={dimensiones} miniatura={entrada.miniatura} />
          </span>
        )}
      </button>

      <button
        type="button"
        className="lamina__eliminar boton-icono"
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

      <div className="lamina__pie">
        {renombrando ? (
          <NombreEditable
            nombre={nombre}
            className="lamina__campo"
            alTerminar={(nuevo, devolver) => acciones.terminarRenombrado(id, nuevo, devolver)}
          />
        ) : (
          <p
            className="lamina__nombre"
            title={`${nombre}\nDoble clic o F2 para renombrar`}
            onClick={() => boton.current?.focus()}
            onDoubleClick={() => acciones.empezarRenombrado(id)}
          >
            {nombre}
          </p>
        )}
        <p className="lamina__meta" id={idMeta}>
          <span className="cifras" aria-hidden="true" title="Ancho × alto × largo, en bloques">
            {formatoDimensiones(dimensiones, true)}
          </span>
          <span className="solo-lectores">{dimensionesHabladas(dimensiones)}, </span>
          <span className="lamina__bloques">
            <Cifra valor={totalBloques} /> {totalBloques === 1 ? "bloque" : "bloques"}
          </span>
          <span className="solo-lectores">, {formatoCorto(formato)}</span>
        </p>
      </div>
    </m.li>
  );
});
