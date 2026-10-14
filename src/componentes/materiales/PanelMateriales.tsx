// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  materiales,
  type ListaMateriales,
  type Region,
} from "../../api/biblioteca";
import { normalizarError } from "../../api/llamar";
import { useBiblioteca } from "../../estado/biblioteca";
import { useAjustes } from "../../estado/ajustes";
import { useEdicion } from "../../estado/edicion";
import { useMateriales } from "../../estado/materiales";
import { useTexturas } from "../../estado/texturas";
import { capaMax, capaMin, useVisor } from "../../estado/visor";
import { Cifra } from "../Cifra";
import { CifraAnimada } from "../CifraAnimada";
import { formatoNumero } from "../../logica/formato";
import {
  aFilas,
  capasAMateriales,
  yDeCapa,
  filtrarYOrdenar,
  textoLista,
  type CampoOrdenMateriales,
  type OrdenMateriales,
} from "../../logica/materiales";
import { BarraFiltros } from "./BarraFiltros";
import { Lista } from "./TablaMateriales";
import "./PanelMateriales.css";

/** Espera tras el último cambio de filtro antes de recalcular (ms). */
const ESPERA_CALCULO_MS = 140;

/** Lista de comprobación vacía (mientras no es la de este schematic). */
const NADA_REUNIDO: ReadonlySet<string> = new Set();

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
  const cargarComprobacion = useMateriales((s) => s.cargarComprobacion);
  const alternarReunidoDe = useMateriales((s) => s.alternarReunido);
  const exportarCsv = useMateriales((s) => s.exportarCsv);
  const reunidos = useMateriales((s) =>
    s.comprobacion?.id === id ? s.comprobacion.reunidos : NADA_REUNIDO,
  );

  const [region, setRegion] = useState<string | null>(null);
  const [soloVisibles, setSoloVisibles] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const [orden, setOrden] = useState<OrdenMateriales>({ campo: "cantidad", ascendente: false });
  const [ocultarReunidos, setOcultarReunidos] = useState(false);
  const [calculo, setCalculo] = useState<Calculo>({ estado: "cargando" });
  const [calculando, setCalculando] = useState(false);

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

  // ── Lista de comprobación (en el almacén de materiales) ──
  useEffect(() => cargarComprobacion(id), [id, cargarComprobacion]);

  // Estable mientras no cambie el schematic: no rompe el memo de las filas.
  const alternarReunido = useCallback(
    (item: string, nombre: string, reunido: boolean) =>
      alternarReunidoDe(id, item, nombre, reunido),
    [id, alternarReunidoDe],
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
    } catch {
      avisar("No se pudo copiar al portapapeles.");
    }
  };

  const hayLista = !!lista && lista.materiales.length > 0;

  return (
    <div className="materiales" aria-busy={calculando}>
      <BarraFiltros
        busqueda={busqueda}
        setBusqueda={setBusqueda}
        orden={orden}
        alternarOrden={alternarOrden}
        ocultarReunidos={ocultarReunidos}
        setOcultarReunidos={setOcultarReunidos}
        regiones={regiones}
        region={region}
        setRegion={setRegion}
        soloVisibles={soloVisibles}
        setSoloVisibles={setSoloVisibles}
        rotuloCapas={rotuloCapas}
      />

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
          onClick={() => void exportarCsv(titulo, filas, reunidos)}
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
