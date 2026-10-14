// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Lista de bloques parecidos al pinchado (o al origen de una fila de la familia).

import { useLayoutEffect, useMemo, useRef, type KeyboardEvent } from "react";
import { m } from "motion/react";
import type { Parecido } from "../../api/parecidos";
import { useEdicion, type ListaParecidos as Lista } from "../../estado/edicion";
import { muelle } from "../../estilos/movimiento";
import { formatoDistancia } from "../../logica/formato";
import { IconoBloque, useNombre } from "./BloqueEdicion";
import "./Parecidos.css";

/** La sección de la familia se abrió con su botón: al montarse se lleva el foco. */
let enfocarFamilia = false;

/** Si la sección de la familia debe llevarse el foco al montarse (y lo olvida). */
export function tomarFocoFamilia(): boolean {
  const enfocar = enfocarFamilia;
  enfocarFamilia = false;
  return enfocar;
}

/**
 * Lista de parecidos de un block state. Al abrirse recuerda quién tenía el
 * foco (el lienzo, o la fila de la tabla de una familia) y, si al cerrarse el
 * foco estaba dentro, se lo devuelve.
 */
export function ListaParecidos({ lista }: { lista: Lista }) {
  const elegir = useEdicion((s) => s.elegirParecido);
  const cerrar = useEdicion((s) => s.cerrarLista);
  const nombreOrigen = useNombre(lista.origen);
  const ul = useRef<HTMLUListElement>(null);
  const raiz = useRef<HTMLElement>(null);
  const apertura = useRef<HTMLElement | null>(null);
  const focoDentro = useRef(false);

  // Destino que ya está pendiente para este objetivo (se marca en la lista).
  // El selector devuelve solo referencias del almacén (barato en cada cambio
  // suyo); la búsqueda, sobre hasta un millón de cambios, va en useMemo.
  const cambios = useEdicion((s) => s.cambios);
  const reemplazos = useEdicion((s) => s.reemplazos);
  const elegido = useMemo(() => {
    const o = lista.objetivo;
    if (o.tipo === "reemplazo")
      return reemplazos.find((r) => r.estadoOrigen === o.estadoOrigen)?.estadoDestino ?? null;
    const [x, y, z] = o.pinchado.bloque.posicion;
    const region = o.pinchado.bloque.region;
    return (
      cambios.find((c) => c.region === region && c.x === x && c.y === y && c.z === z)
        ?.nuevoEstado ?? null
    );
  }, [lista.objetivo, cambios, reemplazos]);

  useLayoutEffect(() => {
    apertura.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => {
      const a = apertura.current;
      const perdido = !document.activeElement || document.activeElement === document.body;
      if ((focoDentro.current || perdido) && a?.isConnected) a.focus({ preventScroll: true });
    };
  }, []);

  // Flechas, Inicio y Fin recorren la lista.
  const alPulsar = (e: KeyboardEvent<HTMLUListElement>) => {
    const botones = [...(ul.current?.querySelectorAll<HTMLButtonElement>("button") ?? [])];
    const i = botones.indexOf(document.activeElement as HTMLButtonElement);
    let j: number;
    if (e.key === "ArrowDown") j = Math.min(i + 1, botones.length - 1);
    else if (e.key === "ArrowUp") j = Math.max(i - 1, 0);
    else if (e.key === "Home") j = 0;
    else if (e.key === "End") j = botones.length - 1;
    else return;
    e.preventDefault();
    botones[j]?.focus();
  };

  const bloque = lista.objetivo.tipo === "bloque" ? lista.objetivo.pinchado.bloque : null;

  return (
    <section
      ref={raiz}
      className="parecidos"
      aria-labelledby="titulo-parecidos"
      onFocus={() => {
        focoDentro.current = true;
      }}
      onBlur={(e) => {
        if (!raiz.current?.contains(e.relatedTarget as Node | null)) focoDentro.current = false;
      }}
    >
      <header className="parecidos__cabecera">
        <IconoBloque estado={lista.origen} />
        <div className="parecidos__origen">
          <h2 id="titulo-parecidos" className="parecidos__nombre">
            {nombreOrigen}
          </h2>
          <p className="parecidos__estado cifras" title={lista.origen}>
            {bloque
              ? `${bloque.posicion.join(", ")}${bloque.region ? ` · ${bloque.region}` : ""}`
              : lista.origen}
          </p>
        </div>
        <button
          type="button"
          className="parecidos__cerrar"
          aria-label="Cerrar la lista (Esc)"
          title="Cerrar (Esc)"
          aria-keyshortcuts="Escape"
          onClick={cerrar}
        >
          <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
            <path d="M1 1L9 9M9 1L1 9" />
          </svg>
        </button>
      </header>

      {bloque && (
        <div className="parecidos__familia">
          <button
            type="button"
            className="boton-plano"
            onClick={() => {
              enfocarFamilia = true;
              useEdicion.getState().seleccionarFamilia(lista.origen);
            }}
          >
            Seleccionar familia
          </button>
          <span className="parecidos__pista">
            o <kbd>Mayús</kbd>+clic en el modelo: todas sus variantes a la vez
          </span>
        </div>
      )}

      {/* Región viva siempre montada: un role="status" que nace con su texto no se anuncia. */}
      <div role="status">
        {lista.estado === "cargando" && (
          <p className="edicion__ayuda">Buscando bloques parecidos…</p>
        )}
        {lista.estado === "lista" && lista.parecidos.length === 0 && (
          <p className="edicion__ayuda">
            No hay bloques de la misma forma con los que compararlo (puede ser de un mod o no tener
            color).
          </p>
        )}
      </div>
      {lista.estado === "error" && (
        <p className="edicion__error" role="alert">
          {lista.mensaje}
        </p>
      )}
      {lista.estado === "lista" && lista.parecidos.length > 0 && (
        <>
          <p className="parecidos__titulo">
            <span className="rotulo">Parecidos</span>
            <span className="parecidos__leyenda">distancia de color</span>
          </p>
          <ul
            ref={ul}
            className="parecidos__lista"
            aria-label={`Bloques parecidos a ${nombreOrigen}`}
            onKeyDown={alPulsar}
          >
            {lista.parecidos.map((p, i) => (
              <FilaParecido
                key={p.estado}
                parecido={p}
                indice={i}
                elegido={p.estado === elegido}
                onElegir={elegir}
              />
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

function FilaParecido({
  parecido,
  indice,
  elegido,
  onElegir,
}: {
  parecido: Parecido;
  indice: number;
  elegido: boolean;
  onElegir: (p: Parecido) => void;
}) {
  const nombre = useNombre(parecido.estado);
  return (
    <m.li
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{
        ...muelle("pulsacion"),
        delay: Math.min(indice, 12) * 0.015,
      }}
    >
      <button
        type="button"
        className="parecido"
        aria-pressed={elegido}
        onClick={() => onElegir(parecido)}
        title={parecido.estado}
      >
        <IconoBloque estado={parecido.estado} color={parecido.color} />
        <span className="parecido__nombre">{nombre}</span>
        <span className="parecido__distancia cifras">
          <span className="edicion__solo-lectores">distancia </span>
          {formatoDistancia(parecido.distancia)}
        </span>
      </button>
    </m.li>
  );
}
