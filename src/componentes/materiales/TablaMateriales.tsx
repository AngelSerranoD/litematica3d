// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { memo, useLayoutEffect, useMemo, useRef, type CSSProperties, type KeyboardEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { type AtlasIconos, type IconoItem } from "../../estado/materiales";
import { formatoNumero } from "../../logica/formato";
import { type Fila } from "../../logica/materiales";
import { CifraAnimada } from "../CifraAnimada";

/** Alto fijo de cada fila de la lista (px): la virtualización cuenta con él. */
const ALTO_FILA = 32;
/** Lado del icono en pantalla (px). */
const LADO_ICONO = 16;
/** Filas que saltan RePág / AvPág en la lista. */
const SALTO_PAGINA = 10;

interface PropsLista {
  filas: Fila[];
  reunidos: ReadonlySet<string>;
  iconos: Map<string, IconoItem> | null;
  atlas: AtlasIconos | null;
  alternar: (item: string, nombre: string, reunido: boolean) => void;
  /** Texto si no hay filas; `null` mientras no hay lista. */
  vacia: string | null;
}

/** Lista virtualizada: solo existen en el DOM las filas que se ven (y unas pocas más). */
export function Lista({ filas, reunidos, iconos, atlas, alternar, vacia }: PropsLista) {
  const contenedor = useRef<HTMLDivElement>(null);
  const virtualizador = useVirtualizer({
    count: filas.length,
    getScrollElement: () => contenedor.current,
    estimateSize: () => ALTO_FILA,
    overscan: 8,
    getItemKey: (i) => filas[i].item,
  });

  // Mover el foco con el teclado: la fila destino puede no existir aún en el
  // DOM; se desplaza hasta ella y se enfoca en cuanto se pinta.
  const pendiente = useRef<number | null>(null);
  useLayoutEffect(() => {
    if (pendiente.current === null) return;
    const destino = contenedor.current?.querySelector<HTMLInputElement>(
      `input[data-indice="${pendiente.current}"]`,
    );
    if (destino) {
      destino.focus({ preventScroll: true });
      pendiente.current = null;
    }
  });

  const alPulsar = (e: KeyboardEvent<HTMLDivElement>) => {
    const indice = Number((e.target as HTMLElement).dataset.indice);
    if (Number.isNaN(indice) || filas.length === 0) return;
    const ultimo = filas.length - 1;
    const destinos: Record<string, number> = {
      ArrowDown: Math.min(indice + 1, ultimo),
      ArrowUp: Math.max(indice - 1, 0),
      PageDown: Math.min(indice + SALTO_PAGINA, ultimo),
      PageUp: Math.max(indice - SALTO_PAGINA, 0),
      Home: 0,
      End: ultimo,
    };
    const destino = destinos[e.key];
    if (destino === undefined) return;
    e.preventDefault();
    pendiente.current = destino;
    virtualizador.scrollToIndex(destino, { align: "auto" });
    const visible = contenedor.current?.querySelector<HTMLInputElement>(
      `input[data-indice="${destino}"]`,
    );
    if (visible) {
      visible.focus({ preventScroll: true });
      pendiente.current = null;
    }
  };

  const estiloAtlas = useMemo<CSSProperties | undefined>(() => {
    if (!atlas) return undefined;
    const k = LADO_ICONO / atlas.tamTextura;
    return {
      "--atlas-imagen": `url("${atlas.url}")`,
      "--atlas-tamano": `${atlas.ancho * k}px ${atlas.alto * k}px`,
      "--atlas-escala": k,
    } as CSSProperties;
  }, [atlas]);

  return (
    <div className="materiales__tabla">
      <div className="materiales__cabecera rotulo" aria-hidden="true">
        <span className="materiales__col-check">✓</span>
        <span />
        <span>Material</span>
        <span className="materiales__num">Cantidad</span>
        <span className="materiales__num">Stacks</span>
        <span className="materiales__num">Shulk.</span>
      </div>
      <div
        ref={contenedor}
        className="materiales__lista"
        style={estiloAtlas}
        role="list"
        aria-label="Materiales"
        onKeyDown={alPulsar}
      >
        {vacia !== null && filas.length === 0 && <p className="materiales__vacia">{vacia}</p>}
        <div className="materiales__alto" style={{ height: virtualizador.getTotalSize() }}>
          {virtualizador.getVirtualItems().map((v) => {
            const f = filas[v.index];
            return (
              <FilaMaterial
                key={v.key}
                fila={f}
                indice={v.index}
                total={filas.length}
                arriba={v.start}
                reunido={reunidos.has(f.item)}
                icono={iconos?.get(f.item) ?? null}
                conAtlas={!!atlas}
                alternar={alternar}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

interface PropsFila {
  fila: Fila;
  indice: number;
  total: number;
  arriba: number;
  reunido: boolean;
  icono: IconoItem | null;
  conAtlas: boolean;
  alternar: (item: string, nombre: string, reunido: boolean) => void;
}

const FilaMaterial = memo(function FilaMaterial({
  fila: f,
  indice,
  total,
  arriba,
  reunido,
  icono,
  conAtlas,
  alternar,
}: PropsFila) {
  const resto = f.stacksEnShulker > 0 || f.sueltosEnShulker > 0;
  const textoShulkers = `${formatoNumero(f.shulkers)} shulkers${
    resto
      ? ` + ${formatoNumero(f.stacksEnShulker)} stacks y ${formatoNumero(f.sueltosEnShulker)} sueltos`
      : ""
  }`;
  return (
    <label
      className={reunido ? "materiales__fila materiales__fila--reunido" : "materiales__fila"}
      role="listitem"
      aria-posinset={indice + 1}
      aria-setsize={total}
      style={{ transform: `translateY(${arriba}px)` }}
    >
      <input
        type="checkbox"
        className="casilla-verificacion"
        checked={reunido}
        data-indice={indice}
        aria-label={`${f.nombre}: ${formatoNumero(f.cantidad)}. Reunido`}
        onChange={(e) => alternar(f.item, f.nombre, e.target.checked)}
      />
      <Icono icono={icono} conAtlas={conAtlas} />
      <span className="materiales__nombre" title={f.item}>
        {f.nombre}
      </span>
      <CifraAnimada valor={f.cantidad} className="materiales__num" />
      <span
        className="materiales__num"
        title={`${formatoNumero(f.stacks)} stacks de ${f.tamanoStack} + ${formatoNumero(f.sueltos)} sueltos`}
      >
        <CifraAnimada valor={f.stacks} />
        {f.sueltos > 0 && (
          <span className="materiales__sueltos">
            +<CifraAnimada valor={f.sueltos} />
          </span>
        )}
      </span>
      <span className="materiales__num" title={textoShulkers}>
        {f.shulkers === 0 && resto ? (
          // Cabe en menos de una caja: «0+» se leería como «nada».
          <span className="cifras materiales__menos">&lt;1</span>
        ) : (
          <CifraAnimada valor={f.shulkers} />
        )}
        <span className="materiales__resto" aria-hidden="true">
          {resto && f.shulkers > 0 ? "+" : ""}
        </span>
      </span>
    </label>
  );
});

/** Textura del bloque si hay atlas; si no, su color plano; si nada, un cuadrado neutro. */
function Icono({ icono, conAtlas }: { icono: IconoItem | null; conAtlas: boolean }) {
  if (!icono)
    return <span className="materiales__icono materiales__icono--neutro" aria-hidden="true" />;
  const { textura } = icono;
  return (
    <span
      className={
        textura && conAtlas ? "materiales__icono materiales__icono--textura" : "materiales__icono"
      }
      aria-hidden="true"
      style={
        textura && conAtlas
          ? {
              backgroundPosition: `calc(${-textura.x}px * var(--atlas-escala)) calc(${-textura.y}px * var(--atlas-escala))`,
            }
          : { backgroundColor: icono.color }
      }
    />
  );
}
