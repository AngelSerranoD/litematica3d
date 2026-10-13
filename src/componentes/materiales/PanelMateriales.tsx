// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import {
  memo,
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  materiales,
  type ListaMateriales,
  type Region,
} from "../../api/biblioteca";
import { normalizarError } from "../../api/llamar";
import { comprobacion, guardarCsv, marcar } from "../../api/materiales";
import { useBiblioteca } from "../../estado/biblioteca";
import { useAjustes } from "../../estado/ajustes";
import { useEdicion } from "../../estado/edicion";
import { useMateriales, type AtlasIconos, type IconoItem } from "../../estado/materiales";
import { useTexturas } from "../../estado/texturas";
import { capaMax, capaMin, useVisor } from "../../estado/visor";
import { Cifra } from "../Cifra";
import { CifraAnimada } from "../CifraAnimada";
import { formatoNumero } from "../formato";
import {
  aFilas,
  capasAMateriales,
  yDeCapa,
  csvLista,
  filtrarYOrdenar,
  nombreArchivo,
  textoLista,
  type CampoOrdenMateriales,
  type Fila,
  type OrdenMateriales,
} from "./logica";
import "./PanelMateriales.css";

/** Alto fijo de cada fila de la lista (px): la virtualización cuenta con él. */
const ALTO_FILA = 32;
/** Espera tras el último cambio de filtro antes de recalcular (ms). */
const ESPERA_CALCULO_MS = 140;
/** Lado del icono en pantalla (px). */
const LADO_ICONO = 16;
/** Filas que saltan RePág / AvPág en la lista. */
const SALTO_PAGINA = 10;

const ORDENES: { campo: CampoOrdenMateriales; etiqueta: string }[] = [
  { campo: "cantidad", etiqueta: "Cantidad" },
  { campo: "nombre", etiqueta: "Nombre" },
];

type Calculo =
  | { estado: "cargando" }
  | { estado: "error"; mensaje: string }
  | { estado: "listo"; lista: ListaMateriales };

interface Props {
  id: string;
  /** Nombre del schematic: título del texto copiado y del CSV. */
  titulo: string;
  regiones: Region[];
}

/**
 * Lista de materiales del schematic abierto, con sus filtros (texto, región
 * y capas del visor), la lista de comprobación y la exportación.
 */
export function PanelMateriales({ id, titulo, regiones }: Props) {
  const avisar = useBiblioteca((s) => s.avisar);
  const nombres = useMateriales((s) => s.nombres);
  const cargarNombres = useMateriales((s) => s.cargarNombres);
  // Con otras texturas, los nombres se vuelven a pedir (otra versión).
  const generacionTexturas = useTexturas((s) => s.generacion);
  // Agua y lava: por defecto no cuentan (se activa en Ajustes).
  const incluirFluidos = useAjustes((s) => s.guardados.incluirFluidos);
  const modelo = useMateriales((s) => (s.modelo?.id === id ? s.modelo : null));
  const extras = useMateriales((s) => s.extras);
  const completarIconos = useMateriales((s) => s.completarIconos);

  const [region, setRegion] = useState<string | null>(null);
  const [soloVisibles, setSoloVisibles] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const [orden, setOrden] = useState<OrdenMateriales>({ campo: "cantidad", ascendente: false });
  const [ocultarReunidos, setOcultarReunidos] = useState(false);
  const [calculo, setCalculo] = useState<Calculo>({ estado: "cargando" });
  const [calculando, setCalculando] = useState(false);
  const [reunidos, setReunidos] = useState<ReadonlySet<string>>(new Set());

  // ── Capas del visor ──
  const desde = useVisor(capaMin);
  const hasta = useVisor(capaMax);
  const arrastrando = useVisor((s) => s.arrastrando);
  // Hasta que el visor no publica el modelo de ESTE schematic, el estado de
  // capas puede ser el del anterior: no se filtra.
  const capas = soloVisibles && modelo ? capasAMateriales(desde, hasta) : null;
  const capaDesde = capas?.[0] ?? null;
  const capaHasta = capas?.[1] ?? null;
  /** «Y 5–44» en coordenadas del schematic, como rotula el deslizador. */
  const rotuloCapas =
    modelo && capaDesde !== null && capaHasta !== null
      ? `Y ${yDeCapa(capaDesde, modelo.origenY)}–${yDeCapa(capaHasta, modelo.origenY)}`
      : null;

  useEffect(() => cargarNombres(), [cargarNombres, generacionTexturas]);

  // La región elegida puede no existir al cambiar de schematic.
  useEffect(() => {
    if (region !== null && !regiones.some((r) => r.nombre === region)) setRegion(null);
  }, [regiones, region]);

  // ── Cálculo: con espera, sin lanzar nada mientras se arrastra el deslizador
  // y descartando las respuestas que ya no corresponden al último filtro. ──
  const ultima = useRef(0);
  // Tras aplicar o restaurar una edición la copia cambió: se vuelve a calcular.
  const revision = useEdicion((s) => s.revision);
  useEffect(() => {
    if (arrastrando) return;
    const temporizador = window.setTimeout(() => {
      const mia = ++ultima.current;
      setCalculando(true);
      const rango: [number, number] | null =
        capaDesde !== null && capaHasta !== null ? [capaDesde, capaHasta] : null;
      materiales(id, {
        incluirFluidos,
        region,
        yMin: null,
        yMax: null,
        capas: rango,
      })
        .then((lista) => {
          if (mia === ultima.current) setCalculo({ estado: "listo", lista });
        })
        .catch((e) => {
          if (mia === ultima.current)
            setCalculo({ estado: "error", mensaje: normalizarError(e).mensaje });
        })
        .finally(() => {
          if (mia === ultima.current) setCalculando(false);
        });
    }, ESPERA_CALCULO_MS);
    return () => window.clearTimeout(temporizador);
  }, [id, region, capaDesde, capaHasta, arrastrando, incluirFluidos, revision]);

  // Al desmontar, cualquier respuesta en vuelo se descarta.
  useEffect(() => () => void ++ultima.current, []);

  // ── Lista de comprobación ──
  useEffect(() => {
    let activo = true;
    setReunidos(new Set());
    comprobacion(id)
      .then((items) => activo && setReunidos(new Set(items)))
      .catch(
        (e) =>
          activo &&
          avisar(`No se pudo leer la lista de comprobación: ${normalizarError(e).mensaje}`),
      );
    return () => {
      activo = false;
    };
  }, [id, avisar]);

  /** Marca o desmarca al momento; si el guardado falla, deshace ese ítem. */
  const alternarReunido = useCallback(
    (item: string, nombre: string, reunido: boolean) => {
      const fijar = (valor: boolean) =>
        setReunidos((previos) => {
          if (previos.has(item) === valor) return previos;
          const nuevos = new Set(previos);
          if (valor) nuevos.add(item);
          else nuevos.delete(item);
          return nuevos;
        });
      fijar(reunido);
      marcar(id, item, reunido).catch((e) => {
        fijar(!reunido);
        avisar(`No se pudo guardar «${nombre}»: ${normalizarError(e).mensaje}`);
      });
    },
    [id, avisar],
  );

  // ── Filas ──
  const lista = calculo.estado === "listo" ? calculo.lista : null;
  const todas = useMemo(() => (lista ? aFilas(lista.materiales, nombres) : []), [lista, nombres]);
  // Los ítems que no son un bloque de la malla (el destino de una regla, como
  // `dirt` para un `dirt_path`) piden su icono por id.
  useEffect(() => {
    if (modelo && todas.length) completarIconos(todas.map((f) => f.item));
  }, [modelo, todas, completarIconos]);
  const iconos = useMemo(() => {
    if (!modelo) return null;
    if (extras?.modelo !== modelo || !extras.iconos.size) return modelo.iconos;
    return new Map([...extras.iconos, ...modelo.iconos]);
  }, [modelo, extras]);
  const filas = useMemo(
    () => filtrarYOrdenar(todas, busqueda, orden, ocultarReunidos, reunidos),
    [todas, busqueda, orden, ocultarReunidos, reunidos],
  );
  const totalReunidos = useMemo(
    () => todas.reduce((n, f) => n + (reunidos.has(f.item) ? 1 : 0), 0),
    [todas, reunidos],
  );

  const alternarOrden = (campo: CampoOrdenMateriales) =>
    setOrden((o) =>
      o.campo === campo
        ? { campo, ascendente: !o.ascendente }
        : { campo, ascendente: campo === "nombre" },
    );

  // ── Exportar ──
  const descripcionFiltro = () => {
    const partes = [`Región: ${region ?? "todas"}`];
    partes.push(rotuloCapas ? `Capas ${rotuloCapas}` : "Todas las capas");
    if (busqueda.trim()) partes.push(`Búsqueda: «${busqueda.trim()}»`);
    if (ocultarReunidos) partes.push("Sin lo reunido");
    return partes.join(" · ");
  };

  const copiar = async () => {
    if (!lista) return;
    const texto = textoLista(filas, reunidos, {
      titulo,
      filtro: descripcionFiltro(),
      totalItems: lista.totalItems,
      totalStacks: lista.totalStacks,
      totalShulkers: lista.totalShulkers,
    });
    try {
      await navigator.clipboard.writeText(texto);
      avisar(`Lista copiada al portapapeles (${formatoNumero(filas.length)} materiales).`);
    } catch (e) {
      avisar(`No se pudo copiar: ${normalizarError(e).mensaje}`);
    }
  };

  const alGuardarCsv = async () => {
    try {
      const guardado = await guardarCsv(nombreArchivo(titulo), csvLista(filas, reunidos));
      if (!guardado) return;
      avisar("Lista de materiales guardada en CSV.");
    } catch (e) {
      avisar(`No se pudo guardar el CSV: ${normalizarError(e).mensaje}`);
    }
  };

  const hayLista = !!lista && lista.materiales.length > 0;

  return (
    <div className="materiales" aria-busy={calculando}>
      <div className="materiales__filtros">
        <label className="materiales__buscar">
          <span className="solo-lectores">Buscar material</span>
          <input
            type="search"
            value={busqueda}
            placeholder="Buscar por nombre o ID"
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => setBusqueda(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape" && busqueda) {
                // Esc limpia la búsqueda; preventDefault para no volver a la biblioteca.
                e.preventDefault();
                setBusqueda("");
              }
            }}
          />
        </label>

        <div className="materiales__fila-filtros">
          <div className="materiales__grupo" role="group" aria-label="Ordenar">
            <span className="materiales__etiqueta rotulo" aria-hidden="true">
              Orden
            </span>
            {ORDENES.map(({ campo, etiqueta }) => {
              const activo = orden.campo === campo;
              const sentido = orden.ascendente ? "ascendente" : "descendente";
              return (
                <button
                  key={campo}
                  type="button"
                  className="materiales__casilla"
                  aria-pressed={activo}
                  aria-label={
                    activo
                      ? `${etiqueta}: orden ${sentido}. Pulsa para invertir`
                      : `Ordenar por ${etiqueta.toLowerCase()}`
                  }
                  onClick={() => alternarOrden(campo)}
                >
                  {etiqueta}
                  <span className="materiales__sentido" aria-hidden="true">
                    {activo && (orden.ascendente ? "↑" : "↓")}
                  </span>
                </button>
              );
            })}
          </div>
          <label className="materiales__interruptor">
            <input
              type="checkbox"
              className="casilla-verificacion"
              checked={ocultarReunidos}
              onChange={(e) => setOcultarReunidos(e.target.checked)}
            />
            Ocultar reunidos
          </label>
        </div>

        <div className="materiales__fila-filtros">
          {regiones.length > 1 && (
            <label className="materiales__region">
              <span className="materiales__etiqueta rotulo">Región</span>
              <select
                value={region ?? ""}
                onChange={(e) => setRegion(e.target.value === "" ? null : e.target.value)}
              >
                <option value="">Todas</option>
                {regiones.map((r) => (
                  <option key={r.nombre} value={r.nombre}>
                    {r.nombre}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label className="materiales__interruptor">
            <input
              type="checkbox"
              className="casilla-verificacion"
              checked={soloVisibles}
              onChange={(e) => setSoloVisibles(e.target.checked)}
            />
            Solo capas visibles
            <span className="materiales__capas cifras" aria-live="polite">
              {rotuloCapas ?? (soloVisibles ? "…" : "todas")}
            </span>
          </label>
        </div>
      </div>

      <div className="materiales__totales">
        <Total etiqueta="Bloques" valor={lista?.totalItems} />
        <Total etiqueta="Stacks" valor={lista?.totalStacks} />
        <Total etiqueta="Shulkers" valor={lista?.totalShulkers} />
      </div>

      <p className="materiales__estado" role="status">
        {calculo.estado === "error" ? (
          <span className="materiales__error">No se pudo calcular: {calculo.mensaje}</span>
        ) : lista ? (
          <>
            <span>
              <Cifra valor={totalReunidos} /> / <Cifra valor={lista.materiales.length} /> reunidos
            </span>
            {lista.desconocidos > 0 && (
              <span className="materiales__desconocidos">
                <Cifra valor={lista.desconocidos} /> desconocidos
              </span>
            )}
            {arrastrando ? (
              <span className="materiales__aviso">Se recalcula al soltar</span>
            ) : (
              calculando && <span className="materiales__aviso">Recalculando…</span>
            )}
          </>
        ) : (
          <span>Calculando materiales…</span>
        )}
      </p>

      <Lista
        filas={filas}
        reunidos={reunidos}
        iconos={iconos}
        atlas={modelo?.atlas ?? null}
        alternar={alternarReunido}
        vacia={
          !lista
            ? null
            : lista.materiales.length === 0
              ? "No hay materiales en este rango."
              : "Ningún material coincide con el filtro."
        }
      />

      <div className="materiales__acciones">
        <button
          type="button"
          className="boton-plano"
          disabled={!hayLista}
          onClick={() => void copiar()}
        >
          Copiar lista
        </button>
        <button
          type="button"
          className="boton-plano"
          disabled={!hayLista}
          onClick={() => void alGuardarCsv()}
        >
          Guardar CSV
        </button>
      </div>
    </div>
  );
}

function Total({ etiqueta, valor }: { etiqueta: string; valor: number | undefined }) {
  return (
    <div className="materiales__total">
      <span className="materiales__etiqueta rotulo">{etiqueta}</span>
      {valor === undefined ? (
        <span className="cifras materiales__total-vacio">—</span>
      ) : (
        <CifraAnimada valor={valor} className="materiales__total-cifra" />
      )}
    </div>
  );
}

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
function Lista({ filas, reunidos, iconos, atlas, alternar, vacia }: PropsLista) {
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
