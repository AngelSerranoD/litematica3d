// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Selección por familia: familias destino y tabla origen → destino.

import { useLayoutEffect, useMemo, useRef, type KeyboardEvent } from "react";
import { m } from "motion/react";
import type { FamiliaParecida } from "../../api/parecidos";
import { useEdicion, type SeleccionFamilia } from "../../estado/edicion";
import { agruparMiembros } from "../../estado/edicionPura";
import { muelle } from "../../estilos/movimiento";
import { formatoDistancia, formatoNumero } from "../../logica/formato";
import { IconoBloque } from "./BloqueEdicion";
import { tomarFocoFamilia } from "./ListaParecidos";
import { TablaFamilia } from "./TablaFamilia";
import "./Parecidos.css";

/** `minecraft:dark_oak` o `dark_oak` → «dark oak» (los ids de mods conservan su namespace). */
const nombreFamilia = (f: string) => f.replace(/^minecraft:/, "").replace(/_/g, " ");

/** Flechas, Inicio y Fin recorren los botones de una lista. */
function recorrerLista(e: KeyboardEvent<HTMLUListElement>) {
  const botones = [...e.currentTarget.querySelectorAll<HTMLButtonElement>(":scope > li > button")];
  const i = botones.indexOf(document.activeElement as HTMLButtonElement);
  let j: number;
  if (e.key === "ArrowDown") j = Math.min(i + 1, botones.length - 1);
  else if (e.key === "ArrowUp") j = Math.max(i - 1, 0);
  else if (e.key === "Home") j = 0;
  else if (e.key === "End") j = botones.length - 1;
  else return;
  e.preventDefault();
  botones[j]?.focus();
}

/**
 * Familia del bloque pinchado: cuántos bloques tiene en el schematic (se ven
 * con un contorno acento en el visor), las familias por las que cambiarla y la tabla
 * origen → destino, donde cada destino se puede cambiar con su lista de
 * parecidos. Lo elegido queda pendiente como cualquier otro cambio.
 */
export function SeccionFamilia({ familia }: { familia: SeleccionFamilia }) {
  const cerrar = useEdicion((s) => s.cerrarFamilia);
  const elegir = useEdicion((s) => s.elegirFamiliaDestino);
  const raiz = useRef<HTMLElement>(null);
  const focoDentro = useRef(false);
  // Variantes = filas de la tabla (bloques distintos), no estados con otra orientación.
  const variantes = useMemo(() => agruparMiembros(familia.miembros).length, [familia.miembros]);
  const total = familia.miembros.reduce((t, m) => t + m.cantidad, 0);
  const nombre = familia.familia ? nombreFamilia(familia.familia) : "";

  // Abierta con el botón «Seleccionar familia» (que desaparece), el foco pasa
  // a la sección; con Mayús+clic se queda en el modelo. Al cerrarla con el
  // foco dentro, vuelve a «Seleccionar familia» de la lista del bloque (o a
  // la pestaña si ya no hay bloque).
  useLayoutEffect(() => {
    if (tomarFocoFamilia()) raiz.current?.focus({ preventScroll: true });
    return () => {
      if (!focoDentro.current) return;
      queueMicrotask(() =>
        (
          document.querySelector<HTMLElement>(".parecidos__familia .boton-plano") ??
          document.getElementById("pestana-edicion-cambiar")
        )?.focus({ preventScroll: true }),
      );
    };
  }, []);

  return (
    <section
      ref={raiz}
      className="familia-edicion"
      aria-labelledby="titulo-familia"
      tabIndex={-1}
      onFocus={() => {
        focoDentro.current = true;
      }}
      onBlur={(e) => {
        if (!raiz.current?.contains(e.relatedTarget as Node | null)) focoDentro.current = false;
      }}
    >
      <header className="parecidos__cabecera">
        <IconoBloque estado={familia.origen} />
        <div className="parecidos__origen">
          <h2 id="titulo-familia" className="parecidos__nombre">
            {nombre ? `Familia ${nombre}` : "Familia"}
          </h2>
          <p className="parecidos__estado cifras">
            {familia.estado === "lista"
              ? `${formatoNumero(variantes)} ${variantes === 1 ? "variante" : "variantes"} · ${formatoNumero(total)} ${total === 1 ? "bloque" : "bloques"}`
              : familia.origen}
          </p>
        </div>
        <button
          type="button"
          className="parecidos__cerrar"
          aria-label="Cerrar la familia (Esc)"
          title="Cerrar (Esc)"
          aria-keyshortcuts="Escape"
          onClick={cerrar}
        >
          <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
            <path d="M1 1L9 9M9 1L1 9" />
          </svg>
        </button>
      </header>

      <div role="status">
        {familia.estado === "cargando" && (
          <p className="edicion__ayuda">Buscando la familia y sus bloques en el schematic…</p>
        )}
      </div>
      {familia.estado === "error" && (
        <p className="edicion__error" role="alert">
          {familia.mensaje}
        </p>
      )}
      {familia.estado === "lista" && (
        <>
          <p className="edicion__ayuda">
            {familia.destino
              ? "El modelo ya muestra el cambio. Puedes ajustar cada bloque en la tabla."
              : "Sus bloques van con contorno en el modelo. Elige la familia por la que cambiarlos:"}
          </p>
          {familia.destinos && familia.destinos.length > 0 && (
            <>
              <p className="parecidos__titulo">
                <span className="rotulo">Cambiar por</span>
                <span className="parecidos__leyenda">distancia de color</span>
              </p>
              <ul
                className="parecidos__lista"
                aria-label={`Familias por las que cambiar ${nombre}`}
                onKeyDown={recorrerLista}
              >
                {familia.destinos.map((d, i) => (
                  <FilaFamiliaDestino
                    key={d.familia}
                    destino={d}
                    indice={i}
                    elegida={familia.destino === d.familia}
                    preparando={familia.pidiendoPlan === d.familia}
                    ocupada={familia.pidiendoPlan !== null}
                    onElegir={elegir}
                  />
                ))}
              </ul>
            </>
          )}
          <div role="status">
            {familia.mensaje && <p className="edicion__ayuda">{familia.mensaje}</p>}
          </div>
          <TablaFamilia familia={familia} />
        </>
      )}
    </section>
  );
}

function FilaFamiliaDestino({
  destino,
  indice,
  elegida,
  preparando,
  ocupada,
  onElegir,
}: {
  destino: FamiliaParecida;
  indice: number;
  elegida: boolean;
  preparando: boolean;
  ocupada: boolean;
  onElegir: (familia: string) => void;
}) {
  const [r, g, b] = destino.color;
  const nombre = nombreFamilia(destino.familia);
  const formas = `${destino.formasComunes} ${destino.formasComunes === 1 ? "forma común" : "formas comunes"}`;
  return (
    <m.li
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ ...muelle("pulsacion"), delay: Math.min(indice, 12) * 0.015 }}
    >
      <button
        type="button"
        className="parecido"
        aria-pressed={elegida}
        aria-busy={preparando}
        aria-disabled={ocupada && !preparando}
        onClick={() => {
          if (!ocupada) onElegir(destino.familia);
        }}
        title={elegida ? "Pulsa otra vez para quitar este cambio" : undefined}
      >
        <span
          className="icono-bloque"
          style={{ width: 24, height: 24, backgroundColor: `rgb(${r} ${g} ${b})` }}
          aria-hidden="true"
        />
        <span className="parecido__nombre">
          {nombre}
          <span className="parecido__detalle">{preparando ? "Preparando…" : formas}</span>
        </span>
        <span className="parecido__distancia cifras">
          <span className="edicion__solo-lectores">distancia </span>
          {formatoDistancia(destino.distancia)}
        </span>
      </button>
    </m.li>
  );
}
