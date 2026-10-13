// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useLayoutEffect, useRef, type CSSProperties, type KeyboardEvent } from "react";
import { AnimatePresence, motion } from "motion/react";
import { formatoNumero, nombreFormato } from "../../api/biblioteca";
import type { FamiliaParecida, Parecido, Rgb } from "../../api/parecidos";
import {
  bloquesAfectados,
  celdaIcono,
  numPendientes,
  useEdicion,
  type CambioPendiente,
  type ListaParecidos as Lista,
  type PanelEdicion as Panel,
  type ReemplazoPendiente,
  type SeleccionFamilia,
} from "../../estado/edicion";
import { useMateriales } from "../../estado/materiales";
import { muelle } from "../../estilos/movimiento";
import { CifraAnimada } from "../CifraAnimada";
import { fechaCompleta } from "../formato";
import { nombreItem } from "../materiales/logica";
import "./Edicion.css";

const PANELES: { valor: Panel; etiqueta: string }[] = [
  { valor: "cambiar", etiqueta: "Cambiar" },
  { valor: "historial", etiqueta: "Historial" },
];

/** `minecraft:oak_stairs[facing=north]` → `minecraft:oak_stairs`. */
const idDe = (estado: string) => estado.split("[", 1)[0];

/** Distancia OKLab con coma decimal y tres cifras: «0,012». */
const formatoDistancia = (d: number) => d.toFixed(3).replace(".", ",");

/** Nombre en inglés del bloque de un block state (como en los materiales). */
function useNombre(estado: string): string {
  const nombres = useMateriales((s) => s.nombres);
  return nombreItem(idDe(estado), nombres);
}

/**
 * Icono de un block state: su textura del atlas (por `celdas_atlas`, aunque
 * no esté en el schematic) o, sin atlas, su color medio.
 */
export function IconoBloque({
  estado,
  color,
  lado = 24,
}: {
  estado: string;
  color?: Rgb | null;
  /** Lado en píxeles. */
  lado?: number;
}) {
  const fila = useEdicion((s) => s.celdas[estado]);
  const versionCeldas = useEdicion((s) => s.versionCeldas);
  const atlas = useMateriales((s) => s.modelo?.atlas ?? null);
  // Los estados que no vinieron en una lista de parecidos (p. ej. los de un
  // plan de familia) se piden aquí; el almacén junta las peticiones.
  useEffect(() => {
    if (fila === undefined) useEdicion.getState().pedirCeldas([estado]);
  }, [estado, fila]);
  const celda = celdaIcono(fila);
  if (celda && atlas && atlas.datos.version === versionCeldas) {
    const d = atlas.datos;
    // k escala el atlas entero para que una textura mida `lado`.
    const k = lado / d.tamTextura;
    const x = (celda.celda % d.celdasPorFila) * d.tamCelda + d.relleno;
    const y = Math.floor(celda.celda / d.celdasPorFila) * d.tamCelda + d.relleno;
    const estilo: CSSProperties = {
      width: lado,
      height: lado,
      backgroundImage: `url("${atlas.url}")`,
      backgroundSize: `${d.ancho * k}px ${d.alto * k}px`,
      backgroundPosition: `${-x * k}px ${-y * k}px`,
    };
    return (
      <span className="icono-bloque icono-bloque--textura" style={estilo} aria-hidden="true" />
    );
  }
  const c = celda?.color ?? color ?? null;
  return (
    <span
      className={c ? "icono-bloque" : "icono-bloque icono-bloque--neutro"}
      style={{
        width: lado,
        height: lado,
        backgroundColor: c ? `rgb(${c[0]} ${c[1]} ${c[2]})` : undefined,
      }}
      aria-hidden="true"
    />
  );
}

/** Nombre del bloque con su icono delante. */
function Bloque({ estado, color }: { estado: string; color?: Rgb | null }) {
  const nombre = useNombre(estado);
  return (
    <span className="bloque-edicion">
      <IconoBloque estado={estado} color={color} lado={16} />
      <span className="bloque-edicion__nombre" title={estado}>
        {nombre}
      </span>
    </span>
  );
}

/**
 * Panel de la aside en Modificar: arriba, el bloque pinchado y sus parecidos
 * (o el historial); abajo, siempre a la vista, los pendientes con Aplicar.
 */
export function PanelEdicion() {
  const panel = useEdicion((s) => s.panel);
  const verPanel = useEdicion((s) => s.verPanel);
  const refPestanas = useRef<Record<Panel, HTMLButtonElement | null>>({
    cambiar: null,
    historial: null,
  });
  const cargarNombres = useMateriales((s) => s.cargarNombres);
  useEffect(() => cargarNombres(), [cargarNombres]);

  const alPulsarPestanas = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(e.key)) return;
    e.preventDefault();
    const destino: Panel =
      e.key === "Home"
        ? "cambiar"
        : e.key === "End"
          ? "historial"
          : panel === "cambiar"
            ? "historial"
            : "cambiar";
    verPanel(destino);
    refPestanas.current[destino]?.focus();
  };

  return (
    <div className="edicion">
      <div
        className="schematic__pestanas"
        role="tablist"
        aria-label="Edición"
        onKeyDown={alPulsarPestanas}
      >
        {PANELES.map(({ valor, etiqueta }) => (
          <button
            key={valor}
            ref={(el) => {
              refPestanas.current[valor] = el;
            }}
            type="button"
            role="tab"
            id={`pestana-edicion-${valor}`}
            className="schematic__pestana rotulo"
            aria-selected={panel === valor}
            aria-controls={panel === valor ? `panel-edicion-${valor}` : undefined}
            tabIndex={panel === valor ? 0 : -1}
            onClick={() => verPanel(valor)}
          >
            {etiqueta}
          </button>
        ))}
      </div>

      <div
        className="edicion__cuerpo"
        role="tabpanel"
        id={`panel-edicion-${panel}`}
        aria-labelledby={`pestana-edicion-${panel}`}
      >
        {panel === "cambiar" ? <SeccionCambiar /> : <SeccionHistorial />}
      </div>

      <SeccionPendientes />
    </div>
  );
}

// ── Cambiar: bloque pinchado y lista de parecidos ──

function SeccionCambiar() {
  const lista = useEdicion((s) => s.lista);
  const pinchado = useEdicion((s) => s.pinchado);
  const familia = useEdicion((s) => s.familia);

  if (familia) return <SeccionFamilia familia={familia} />;
  if (!lista && !pinchado)
    return (
      <p className="edicion__ayuda">
        Pincha un bloque del modelo para ver los más parecidos de la misma forma. Lo que elijas
        queda pendiente hasta que pulses <strong>Aplicar</strong>.
      </p>
    );
  if (!lista) return <p className="edicion__ayuda">Leyendo el bloque…</p>;
  return <ListaParecidos lista={lista} />;
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
  const elegido = useEdicion((s) => {
    const o = lista.objetivo;
    if (o.tipo === "reemplazo")
      return s.reemplazos.find((r) => r.estadoOrigen === o.estadoOrigen)?.estadoDestino ?? null;
    const [x, y, z] = o.pinchado.bloque.posicion;
    const region = o.pinchado.bloque.region;
    return (
      s.cambios.find((c) => c.region === region && c.x === x && c.y === y && c.z === z)
        ?.nuevoEstado ?? null
    );
  });

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
    <motion.li
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
    </motion.li>
  );
}

// ── Selección por familia ──

/** La sección de la familia se abrió con su botón: al montarse se lleva el foco. */
let enfocarFamilia = false;

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
 * con una trama en el visor), las familias por las que cambiarla y la tabla
 * origen → destino, donde cada destino se puede cambiar con su lista de
 * parecidos. Lo elegido queda pendiente como cualquier otro cambio.
 */
function SeccionFamilia({ familia }: { familia: SeleccionFamilia }) {
  const cerrar = useEdicion((s) => s.cerrarFamilia);
  const elegir = useEdicion((s) => s.elegirFamiliaDestino);
  const raiz = useRef<HTMLElement>(null);
  const focoDentro = useRef(false);
  const total = familia.miembros.reduce((t, m) => t + m.cantidad, 0);
  const nombre = familia.familia ? nombreFamilia(familia.familia) : "";

  // Abierta con el botón «Seleccionar familia» (que desaparece), el foco pasa
  // a la sección; con Mayús+clic se queda en el modelo. Al cerrarla con el
  // foco dentro, vuelve a «Seleccionar familia» de la lista del bloque (o a
  // la pestaña si ya no hay bloque).
  useLayoutEffect(() => {
    if (enfocarFamilia) raiz.current?.focus({ preventScroll: true });
    enfocarFamilia = false;
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
              ? `${formatoNumero(familia.miembros.length)} ${familia.miembros.length === 1 ? "variante" : "variantes"} · ${formatoNumero(total)} ${total === 1 ? "bloque" : "bloques"}`
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
              : "Sus bloques van tramados en el modelo. Elige la familia por la que cambiarlos:"}
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
    <motion.li
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
    </motion.li>
  );
}

/** Tabla origen → destino de la familia: una fila por estado presente. */
function TablaFamilia({ familia }: { familia: SeleccionFamilia }) {
  const reemplazos = useEdicion((s) => s.reemplazos);
  const lista = useEdicion((s) => s.lista);
  const destinoDe = new Map(reemplazos.map((r) => [r.estadoOrigen, r.estadoDestino]));
  const abierta = lista?.objetivo.tipo === "reemplazo" ? lista.objetivo.estadoOrigen : null;
  const cambian = familia.miembros.filter((m) => destinoDe.has(m.estado)).length;

  return (
    <>
      <p className="parecidos__titulo">
        <span className="rotulo">Origen → destino</span>
        <span className="parecidos__leyenda cifras">
          {formatoNumero(cambian)} de {formatoNumero(familia.miembros.length)} cambian
        </span>
      </p>
      <ul className="tabla-familia" aria-label="Bloques de la familia y por qué se cambian">
        {familia.miembros.map((m) => (
          <FilaFamilia
            key={m.estado}
            estado={m.estado}
            cantidad={m.cantidad}
            destino={destinoDe.get(m.estado) ?? null}
            lista={abierta === m.estado && lista ? lista : null}
          />
        ))}
      </ul>
    </>
  );
}

function FilaFamilia({
  estado,
  cantidad,
  destino,
  lista,
}: {
  estado: string;
  cantidad: number;
  destino: string | null;
  /** Lista de parecidos abierta para esta fila. */
  lista: Lista | null;
}) {
  const nombre = useNombre(estado);
  const nombreDestino = useNombre(destino ?? estado);
  const idLista = `lista-familia-${estado.replace(/[^a-z0-9]+/gi, "-")}`;
  const alternar = () => {
    const e = useEdicion.getState();
    if (lista) e.cerrarLista();
    else e.abrirLista({ tipo: "reemplazo", estadoOrigen: estado });
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
            onClick={() => useEdicion.getState().quitarReemplazo(estado)}
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

// ── Pendientes ──

function SeccionPendientes() {
  const cambios = useEdicion((s) => s.cambios);
  const reemplazos = useEdicion((s) => s.reemplazos);
  const n = useEdicion(numPendientes);
  const ocupado = useEdicion((s) => s.ocupado);
  const error = useEdicion((s) => s.error);
  const { total, exacto } = bloquesAfectados(cambios, reemplazos);

  return (
    <section className="pendientes-edicion" aria-labelledby="titulo-pendientes-edicion">
      <header className="pendientes-edicion__cabecera">
        <h2 id="titulo-pendientes-edicion" className="rotulo pendientes-edicion__titulo">
          Pendientes
        </h2>
        <CifraAnimada valor={n} className="pendientes-edicion__cuenta" />
      </header>

      {n === 0 ? (
        <p className="edicion__ayuda">Nada pendiente.</p>
      ) : (
        <ul className="pendientes-edicion__lista" aria-label="Cambios pendientes">
          <AnimatePresence initial={false}>
            {reemplazos.map((r) => (
              <FilaReemplazo key={`r:${r.estadoOrigen}`} reemplazo={r} />
            ))}
            {cambios.map((c) => (
              <FilaCambio key={`c:${c.clave}`} cambio={c} />
            ))}
          </AnimatePresence>
        </ul>
      )}

      {error && (
        <p className="edicion__error" role="alert">
          {error}
        </p>
      )}

      <div className="pendientes-edicion__acciones">
        <span className="pendientes-edicion__total">
          {n > 0 && (
            <>
              {exacto ? "" : "≥ "}
              <span className="cifras">{formatoNumero(total)}</span>{" "}
              {total === 1 ? "bloque" : "bloques"}
            </>
          )}
        </span>
        <button
          type="button"
          className="boton-plano"
          disabled={n === 0 || ocupado !== null}
          onClick={() => useEdicion.getState().pedirDescartar("todo", () => {})}
        >
          Descartar todo
        </button>
        <button
          type="button"
          className="boton-plano boton-plano--primario"
          disabled={n === 0 || ocupado !== null}
          aria-busy={ocupado === "aplicar"}
          onClick={() => useEdicion.getState().pedirAplicar()}
        >
          {ocupado === "aplicar" ? "Aplicando…" : "Aplicar…"}
        </button>
      </div>
    </section>
  );
}

/** Fila de la lista de pendientes: origen → destino, dónde, y quitar. */
function FilaPendiente({
  origen,
  destino,
  donde,
  onQuitar,
}: {
  origen: string;
  destino: string;
  donde: string;
  onQuitar: () => void;
}) {
  const nombreOrigen = useNombre(origen);
  const nombreDestino = useNombre(destino);
  return (
    <motion.li
      className="pendiente-edicion"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={muelle("pulsacion")}
    >
      <span className="pendiente-edicion__bloques">
        <Bloque estado={origen} />
        <span className="pendiente-edicion__flecha" aria-hidden="true">
          →
        </span>
        <span className="edicion__solo-lectores">por</span>
        <Bloque estado={destino} />
      </span>
      <span className="pendiente-edicion__donde cifras">{donde}</span>
      <button
        type="button"
        className="pendiente-edicion__quitar"
        aria-label={`Quitar: ${nombreOrigen} por ${nombreDestino}, ${donde}`}
        title="Quitar"
        onClick={onQuitar}
      >
        <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
          <path d="M1 1L9 9M9 1L1 9" />
        </svg>
      </button>
    </motion.li>
  );
}

function FilaCambio({ cambio }: { cambio: CambioPendiente }) {
  const quitar = useEdicion((s) => s.quitarCambio);
  return (
    <FilaPendiente
      origen={cambio.estadoAnterior}
      destino={cambio.nuevoEstado}
      donde={`${cambio.x}, ${cambio.y}, ${cambio.z}`}
      onQuitar={() => quitar(cambio.clave)}
    />
  );
}

function FilaReemplazo({ reemplazo }: { reemplazo: ReemplazoPendiente }) {
  const quitar = useEdicion((s) => s.quitarReemplazo);
  const donde =
    reemplazo.cantidad === null
      ? "todos"
      : `${formatoNumero(reemplazo.cantidad)} ${reemplazo.cantidad === 1 ? "bloque" : "bloques"}`;
  return (
    <FilaPendiente
      origen={reemplazo.estadoOrigen}
      destino={reemplazo.estadoDestino}
      donde={donde}
      onQuitar={() => quitar(reemplazo.estadoOrigen)}
    />
  );
}

// ── Historial ──

function SeccionHistorial() {
  const versiones = useEdicion((s) => s.versiones);
  const cargando = useEdicion((s) => s.cargandoHistorial);
  const error = useEdicion((s) => s.errorHistorial);
  const ocupado = useEdicion((s) => s.ocupado);
  const restaurar = useEdicion((s) => s.restaurar);

  // Al montar (p. ej. al volver a la pestaña) se pide otra vez: puede haber cambiado.
  useEffect(() => {
    void useEdicion.getState().cargarHistorial();
  }, []);

  if (error)
    return (
      <p className="edicion__error" role="alert">
        No se pudo leer el historial: {error}
      </p>
    );
  if (!versiones)
    return (
      <p className="edicion__ayuda" role="status">
        {cargando ? "Leyendo el historial…" : ""}
      </p>
    );
  if (versiones.length === 0)
    return (
      <p className="edicion__ayuda">
        Aún no hay versiones anteriores. Cada vez que apliques cambios, la versión de antes se
        guarda aquí (las 20 últimas) y se puede restaurar.
      </p>
    );

  return (
    <ul className="historial" aria-label="Versiones anteriores" aria-busy={cargando}>
      {versiones.map((v) => (
        <li key={v.version} className="historial__fila">
          <div className="historial__datos">
            <p className="historial__fecha">{fechaCompleta(v.fecha)}</p>
            <p className="historial__detalle">
              {v.motivo === "aplicar"
                ? `Antes de cambiar ${formatoNumero(v.cambios)} ${v.cambios === 1 ? "bloque" : "bloques"}`
                : "Antes de restaurar otra versión"}
            </p>
            <p className="historial__detalle cifras">
              {nombreFormato(v.formato)} · {formatoNumero(v.totalBloques)} bloques
            </p>
          </div>
          <button
            type="button"
            className="boton-plano"
            disabled={ocupado !== null}
            aria-label={`Restaurar la versión del ${fechaCompleta(v.fecha)}`}
            onClick={() => void restaurar(v.version)}
          >
            Restaurar
          </button>
        </li>
      ))}
    </ul>
  );
}
