// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import { capaMax, capaMin, useVisor, type Pulgar } from "../../estado/visor";

/** Capas que salta RePág / AvPág. */
const SALTO_PAGINA = 10;

interface Props {
  /** Y del modelo en coordenadas del schematic: las capas se rotulan con ella. */
  origenY: number;
}

/**
 * Corte por capas Y: deslizador vertical de rango doble (arriba la última
 * capa). Cada pulgar es un `role="slider"` con flechas, RePág/AvPág, Inicio y
 * Fin. Con «Solo esta capa» queda un único pulgar que mueve las dos a la vez.
 */
export function DeslizadorCapas({ origenY }: Props) {
  const alto = useVisor((s) => s.alto);
  const desde = useVisor(capaMin);
  const hasta = useVisor(capaMax);
  const soloCapa = useVisor((s) => s.soloCapa);
  const pista = useRef<HTMLDivElement>(null);
  const arrastre = useRef<Pulgar | "capa" | null>(null);

  const max = alto - 1;
  const fijar = (pulgar: Pulgar | "capa", capa: number) => {
    const s = useVisor.getState();
    if (pulgar === "capa") s.fijarCapa(capa);
    else if (pulgar === "desde") s.fijarDesde(capa);
    else s.fijarHasta(capa);
  };

  const capaEnPuntero = (clientY: number) => {
    const r = pista.current?.getBoundingClientRect();
    if (!r || r.height === 0) return 0;
    return Math.round((1 - (clientY - r.top) / r.height) * max);
  };

  const alPulsar = (e: PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0 || max <= 0) return;
    e.preventDefault();
    const capa = capaEnPuntero(e.clientY);
    // Se mueve el pulgar más cercano; empatados, el que lleva hacia fuera.
    const pulgar: Pulgar | "capa" = soloCapa
      ? "capa"
      : Math.abs(capa - desde) < Math.abs(capa - hasta) || (capa < desde && desde === hasta)
        ? "desde"
        : "hasta";
    arrastre.current = pulgar;
    // Captura: el arrastre sigue aunque el puntero salga de la pista.
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Puntero que ya no existe: se arrastra sin captura.
    }
    useVisor.getState().fijarArrastre(true);
    fijar(pulgar, capa);
    // El foco va al pulgar que se mueve, para seguir con el teclado.
    e.currentTarget.querySelector<HTMLElement>(`[data-pulgar="${pulgar}"]`)?.focus();
  };

  const alMover = (e: PointerEvent<HTMLDivElement>) => {
    if (arrastre.current) fijar(arrastre.current, capaEnPuntero(e.clientY));
  };

  const alSoltar = (e: PointerEvent<HTMLDivElement>) => {
    if (!arrastre.current) return;
    if (e.type === "pointerup") fijar(arrastre.current, capaEnPuntero(e.clientY));
    arrastre.current = null;
    useVisor.getState().fijarArrastre(false);
  };

  const teclas = (pulgar: Pulgar | "capa", valor: number) => (e: KeyboardEvent) => {
    const pasos: Record<string, number> = {
      ArrowUp: 1,
      ArrowRight: 1,
      ArrowDown: -1,
      ArrowLeft: -1,
      PageUp: SALTO_PAGINA,
      PageDown: -SALTO_PAGINA,
    };
    let nuevo: number;
    if (e.key in pasos) nuevo = valor + pasos[e.key];
    else if (e.key === "Home") nuevo = 0;
    else if (e.key === "End") nuevo = max;
    else return;
    e.preventDefault();
    fijar(pulgar, nuevo);
  };

  const pct = (capa: number) => (max > 0 ? (capa / max) * 100 : 0);
  const rotulo = (capa: number) => origenY + capa;

  const pulgar = (tipo: Pulgar | "capa", valor: number, nombre: string) => (
    <div
      key={tipo}
      className="capas__pulgar"
      data-pulgar={tipo}
      role="slider"
      tabIndex={0}
      aria-label={nombre}
      aria-orientation="vertical"
      aria-valuemin={rotulo(tipo === "hasta" ? desde : 0)}
      aria-valuemax={rotulo(tipo === "desde" ? hasta : max)}
      aria-valuenow={rotulo(valor)}
      aria-valuetext={`Y ${rotulo(valor)}`}
      style={{ bottom: `${pct(valor)}%` }}
      onKeyDown={teclas(tipo, valor)}
    >
      <span className="capas__rotulo cifras" aria-hidden="true">
        {rotulo(valor)}
      </span>
    </div>
  );

  return (
    <div className="capas" role="group" aria-label="Corte por capas">
      <span className="capas__titulo rotulo" aria-hidden="true">
        Y
      </span>
      <div
        className="capas__pista"
        ref={pista}
        onPointerDown={alPulsar}
        onPointerMove={alMover}
        onPointerUp={alSoltar}
        onPointerCancel={alSoltar}
        onLostPointerCapture={alSoltar}
      >
        <div className="capas__carril" aria-hidden="true">
          <div
            className="capas__rango"
            style={{ bottom: `${pct(desde)}%`, top: `${100 - pct(hasta)}%` }}
          />
        </div>
        {soloCapa
          ? pulgar("capa", desde, "Capa")
          : [pulgar("desde", desde, "Primera capa"), pulgar("hasta", hasta, "Última capa")]}
      </div>
      <button
        type="button"
        className="capas__solo boton-plano"
        aria-pressed={soloCapa}
        onClick={() => useVisor.getState().alternarSoloCapa()}
        disabled={max <= 0}
      >
        Solo esta capa
      </button>
    </div>
  );
}
