// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as EventoTecladoReact,
  type ReactNode,
} from "react";
import { flushSync } from "react-dom";
import { AnimatePresence, motion, useIsPresent } from "motion/react";
import { BarraBiblioteca } from "../componentes/BarraBiblioteca";
import { BibliotecaVacia } from "../componentes/BibliotecaVacia";
import { Cifra } from "../componentes/Cifra";
import type { AccionesElemento, InfoEntrada } from "../componentes/elementoBiblioteca";
import { FilaLista } from "../componentes/FilaLista";
import { IconoSentido } from "../componentes/Iconos";
import { Lamina } from "../componentes/Lamina";
import { ultimoPendiente } from "../componentes/Pendientes";
import {
  useBiblioteca,
  useEntradasVisibles,
  type CampoOrden,
  type Orden,
} from "../estado/biblioteca";
import { useAjustes } from "../estado/ajustes";
import { useFoco } from "../estado/foco";
import { useNavegacion } from "../estado/navegacion";
import { variantePieza, varianteVista } from "./transicion";
import "./vistas.css";
import "./Biblioteca.css";

/**
 * Ids que ya se han pintado alguna vez en esta sesión. Fuera del componente
 * porque la biblioteca se desmonta al abrir un schematic: al volver, lo
 * recién importado no debe «caer» otra vez.
 */
const idsVistos = new Set<string>();

/** Desplazamiento del cuerpo al abrir un schematic, para volver al mismo sitio. */
let desplazamientoGuardado = 0;

/** Entrada escalonada: 18 ms entre láminas y como mucho 24 pasos (≈ 0,4 s). */
const PASO_ESCALONADO = 0.018;
const PASOS_MAXIMOS = 24;

const esEditable = (el: EventTarget | null) =>
  el instanceof HTMLElement &&
  (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);

export function Biblioteca({ soltando }: { soltando: boolean }) {
  const abrirSchematic = useNavegacion((s) => s.abrir);
  const direccion = useNavegacion((s) => s.direccion);
  // La vista que sale no debe recibir clics ni teclas mientras se desvanece.
  const presente = useIsPresent();

  const visibles = useEntradasVisibles();
  const total = useBiblioteca((s) => s.entradas.length);
  const cargando = useBiblioteca((s) => s.cargando);
  const error = useBiblioteca((s) => s.error);
  const busqueda = useBiblioteca((s) => s.busqueda);
  const modoVista = useBiblioteca((s) => s.modoVista);
  const orden = useBiblioteca((s) => s.orden);
  const ultimaImportada = useBiblioteca((s) => s.ultimaImportada);
  const cargar = useBiblioteca((s) => s.cargar);
  const establecerBusqueda = useBiblioteca((s) => s.establecerBusqueda);
  const alternarOrden = useBiblioteca((s) => s.alternarOrden);
  const pedido = useFoco((s) => s.pedido);

  const cuerpo = useRef<HTMLDivElement>(null);
  const contenedor = useRef<HTMLUListElement & HTMLTableElement>(null);
  const buscar = useRef<HTMLInputElement>(null);
  const idAyuda = useId();

  // El schematic del que se vuelve (si se vuelve de uno): llega volando a
  // su lámina, así que esa lámina no hace la entrada escalonada.
  const [idFijo] = useState(() => (direccion === -1 ? useNavegacion.getState().ultimoId : null));
  const [activoId, setActivoId] = useState<string | null>(idFijo);
  const [renombrandoId, setRenombrandoId] = useState<string | null>(null);
  // Elemento con layoutId compartido con la vista Schematic (ver Lamina).
  const [compartidoId, setCompartidoId] = useState<string | null>(idFijo);

  // Tabindex itinerante: un solo elemento de la colección está en la
  // tabulación; si el activo deja de verse, lo es el primero.
  const activo = useMemo(
    () => (visibles.some((e) => e.id === activoId) ? activoId : (visibles[0]?.id ?? null)),
    [visibles, activoId],
  );

  // Tabindex itinerante aplicado en el DOM: cambiar de elemento activo no
  // vuelve a pintar ninguna lámina (cada repintado de un nodo con layout
  // obliga a Motion a medir las 500).
  useLayoutEffect(() => {
    const raiz = cuerpo.current;
    if (!raiz) return;
    for (const b of raiz.querySelectorAll<HTMLElement>('[data-abrir][tabindex="0"]')) {
      if (b.dataset.abrir !== activo) b.tabIndex = -1;
    }
    if (activo) {
      const b = raiz.querySelector<HTMLElement>(`[data-abrir="${CSS.escape(activo)}"]`);
      if (b) b.tabIndex = 0;
    }
  });

  // Lo último de cada render, para las acciones estables y los atajos.
  const ultimo = useRef({ visibles, activo, presente });
  useLayoutEffect(() => {
    ultimo.current = { visibles, activo, presente };
  });

  useEffect(() => {
    if (useBiblioteca.getState().cargando) void cargar();
  }, [cargar]);

  // ── Animación de entrada de cada elemento ──
  const recientes = useMemo(() => new Set(ultimaImportada), [ultimaImportada]);
  const infos = useRef(new Map<string, InfoEntrada>());
  const oleadaHecha = useRef(false);
  const infoEntrada = (id: string, indice: number): InfoEntrada => {
    let info = infos.current.get(id);
    if (!info) {
      info = {
        retraso: oleadaHecha.current ? 0 : Math.min(indice, PASOS_MAXIMOS) * PASO_ESCALONADO,
        caer: recientes.has(id) && !idsVistos.has(id),
        fija: id === idFijo,
      };
      infos.current.set(id, info);
    }
    return info;
  };
  useEffect(() => {
    if (visibles.length) oleadaHecha.current = true;
    for (const e of visibles) idsVistos.add(e.id);
  }, [visibles]);

  // ── Foco ──
  const botonDe = useCallback(
    (id: string) =>
      cuerpo.current?.querySelector<HTMLElement>(`[data-abrir="${CSS.escape(id)}"]`) ?? null,
    [],
  );

  const enfocar = useCallback(
    (id: string) => {
      setActivoId(id);
      botonDe(id)?.focus();
    },
    [botonDe],
  );

  // Al volver de un schematic: mismo desplazamiento y foco en su lámina. En
  // un efecto de layout para que Motion mida el destino del vuelo ya
  // desplazado.
  useLayoutEffect(() => {
    if (!idFijo || !cuerpo.current) return;
    cuerpo.current.scrollTop = desplazamientoGuardado;
    botonDe(idFijo)?.focus({ preventScroll: true });
  }, [idFijo, botonDe]);

  // Peticiones de foco de fuera (Deshacer del aviso de borrado).
  useEffect(() => {
    if (!pedido) return;
    if (visibles.some((e) => e.id === pedido.id)) enfocar(pedido.id);
    useFoco.getState().atender();
  }, [pedido, visibles, enfocar]);

  // ── Acciones (objeto estable: no rompe el memo de láminas y filas) ──
  const acciones = useMemo<AccionesElemento>(
    () => ({
      abrir: (id) => {
        desplazamientoGuardado = cuerpo.current?.scrollTop ?? 0;
        // El layoutId tiene que estar en el DOM antes de que monte la vista
        // Schematic, para que Motion tenga de dónde hacer volar el visor.
        flushSync(() => setCompartidoId(id));
        abrirSchematic(id);
      },
      activar: (id) => setActivoId(id),
      empezarRenombrado: (id) => setRenombrandoId(id),
      terminarRenombrado: (id, nombre, devolverFoco) => {
        setRenombrandoId(null);
        if (nombre) void useBiblioteca.getState().renombrar(id, nombre);
        if (devolverFoco) enfocar(id);
      },
      eliminar: (id) => {
        const lista = ultimo.current.visibles;
        const i = lista.findIndex((e) => e.id === id);
        const siguiente = lista[i + 1] ?? lista[i - 1];
        // El foco pasa al vecino antes de que la lámina desaparezca; si
        // estaba en otra parte (borrado con el ratón), no se toca.
        const conFoco = cuerpo.current?.contains(document.activeElement) ?? false;
        if (siguiente) {
          setActivoId(siguiente.id);
          if (conFoco) botonDe(siguiente.id)?.focus();
        }
        useBiblioteca.getState().eliminar(id);
      },
    }),
    [abrirSchematic, enfocar, botonDe],
  );

  // ── Teclado de la colección ──
  const columnas = (): number => {
    if (modoVista === "lista" || !contenedor.current) return 1;
    const pistas = getComputedStyle(contenedor.current).gridTemplateColumns.split(" ");
    return Math.max(1, pistas.filter(Boolean).length);
  };

  const alPulsarTecla = (e: EventoTecladoReact<HTMLDivElement>) => {
    if (esEditable(e.target) || e.altKey || e.ctrlKey || e.metaKey) return;
    const lista = visibles;
    const i = lista.findIndex((x) => x.id === activo);
    if (i < 0) return;
    const cols = columnas();
    const ultimoIndice = lista.length - 1;
    const alturaFila = modoVista === "lista" ? 40 : 240;
    const filasPorPagina = Math.max(
      1,
      Math.floor((cuerpo.current?.clientHeight ?? alturaFila) / alturaFila) - 1,
    );

    let destino: number | null = null;
    switch (e.key) {
      case "ArrowRight":
        destino = modoVista === "rejilla" ? Math.min(i + 1, ultimoIndice) : null;
        break;
      case "ArrowLeft":
        destino = modoVista === "rejilla" ? Math.max(i - 1, 0) : null;
        break;
      case "ArrowDown":
        // En la última fila incompleta, bajar lleva al último elemento.
        destino =
          i + cols <= ultimoIndice
            ? i + cols
            : Math.floor(i / cols) < Math.floor(ultimoIndice / cols)
              ? ultimoIndice
              : i;
        break;
      case "ArrowUp":
        destino = i - cols >= 0 ? i - cols : i;
        break;
      case "PageDown":
        destino = Math.min(i + cols * filasPorPagina, ultimoIndice);
        break;
      case "PageUp":
        destino = Math.max(i - cols * filasPorPagina, 0);
        break;
      case "Home":
        destino = 0;
        break;
      case "End":
        destino = ultimoIndice;
        break;
      case "F2":
        e.preventDefault();
        setRenombrandoId(lista[i].id);
        return;
      case "Delete":
        e.preventDefault();
        acciones.eliminar(lista[i].id);
        return;
      default:
        return;
    }
    if (destino === null) return;
    e.preventDefault();
    enfocar(lista[destino].id);
  };

  // ── Atajos globales mientras se ve la biblioteca ──
  useEffect(() => {
    const alPulsar = (e: KeyboardEvent) => {
      if (!ultimo.current.presente || e.defaultPrevented || useAjustes.getState().abierto) return;
      const tecla = e.key.toLowerCase();
      const ctrl = e.ctrlKey || e.metaKey;

      if (ctrl && !e.shiftKey && !e.altKey && tecla === "o") {
        e.preventDefault();
        void useBiblioteca.getState().elegirEImportar();
      } else if (ctrl && !e.shiftKey && !e.altKey && tecla === "f") {
        e.preventDefault();
        buscar.current?.focus();
        buscar.current?.select();
      } else if (e.key === "/" && !ctrl && !e.altKey && !esEditable(e.target)) {
        e.preventDefault();
        buscar.current?.focus();
        buscar.current?.select();
      } else if (ctrl && !e.shiftKey && !e.altKey && tecla === "z" && !esEditable(e.target)) {
        const pendiente = ultimoPendiente(useBiblioteca.getState().pendientesDeBorrar);
        if (!pendiente) return;
        e.preventDefault();
        useBiblioteca.getState().deshacerEliminar(pendiente.entrada.id);
        useFoco.getState().pedir(pendiente.entrada.id);
      }
    };
    window.addEventListener("keydown", alPulsar);
    return () => window.removeEventListener("keydown", alPulsar);
  }, []);

  const salirDeBusqueda = useCallback(() => {
    const { activo: id } = ultimo.current;
    if (id) enfocar(id);
    else buscar.current?.blur();
  }, [enfocar]);

  // ── Contenido ──
  const filtrando = busqueda.trim() !== "";
  let contenido: ReactNode = null;
  if (error) {
    contenido = (
      <div className="biblioteca__aviso" role="alert">
        <p>No se pudo leer la biblioteca: {error.mensaje}</p>
        <button type="button" className="boton-plano" onClick={() => void cargar()}>
          Reintentar
        </button>
      </div>
    );
  } else if (cargando) {
    contenido = <p className="biblioteca__aviso">Leyendo la biblioteca…</p>;
  } else if (total === 0) {
    contenido = <BibliotecaVacia soltando={soltando} />;
  } else if (visibles.length === 0) {
    contenido = (
      <div className="biblioteca__aviso">
        <p>Ningún schematic coincide con «{busqueda.trim()}».</p>
        <button
          type="button"
          className="boton-plano"
          onClick={() => {
            establecerBusqueda("");
            buscar.current?.focus();
          }}
        >
          Limpiar la búsqueda
        </button>
      </div>
    );
  } else if (modoVista === "rejilla") {
    contenido = (
      <ul
        ref={contenedor}
        className="rejilla"
        aria-label={`Schematics: ${visibles.length}`}
        aria-describedby={idAyuda}
      >
        <AnimatePresence mode="popLayout">
          {visibles.map((e, i) => (
            <Lamina
              key={e.id}
              entrada={e}
              renombrando={e.id === renombrandoId}
              compartido={e.id === compartidoId}
              info={infoEntrada(e.id, i)}
              acciones={acciones}
            />
          ))}
        </AnimatePresence>
      </ul>
    );
  } else {
    contenido = (
      <table
        ref={contenedor}
        className="lista"
        aria-label={`Schematics: ${visibles.length}`}
        aria-describedby={idAyuda}
      >
        <thead>
          <tr>
            <th scope="col" className="lista__miniatura">
              <span className="solo-lectores">Miniatura</span>
            </th>
            <CabeceraOrdenable
              campo="nombre"
              etiqueta="Nombre"
              orden={orden}
              alPulsar={alternarOrden}
            />
            <th scope="col" className="lista__num">
              Medidas
            </th>
            <CabeceraOrdenable
              campo="tamano"
              etiqueta="Bloques"
              orden={orden}
              alPulsar={alternarOrden}
              numerica
            />
            <th scope="col">Formato</th>
            <CabeceraOrdenable
              campo="fecha"
              etiqueta="Importado"
              orden={orden}
              alPulsar={alternarOrden}
            />
            <th scope="col" className="lista__acciones">
              <span className="solo-lectores">Eliminar</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <AnimatePresence>
            {visibles.map((e, i) => (
              <FilaLista
                key={e.id}
                entrada={e}
                renombrando={e.id === renombrandoId}
                compartido={e.id === compartidoId}
                info={infoEntrada(e.id, i)}
                acciones={acciones}
              />
            ))}
          </AnimatePresence>
        </tbody>
      </table>
    );
  }

  return (
    <motion.section
      className="vista vista--biblioteca"
      aria-labelledby="titulo-biblioteca"
      variants={varianteVista}
      initial="oculta"
      animate="visible"
      exit="saliendo"
      custom={direccion}
      inert={!presente}
    >
      <motion.header className="vista__cabecera" variants={variantePieza} custom={direccion}>
        <h1 id="titulo-biblioteca" className="vista__titulo rotulo">
          Biblioteca
        </h1>
        {!cargando && !error && total > 0 && (
          <p className="vista__dato" aria-live="polite">
            {filtrando ? (
              <>
                <Cifra valor={visibles.length} /> de <Cifra valor={total} />
              </>
            ) : (
              <>
                <Cifra valor={total} /> {total === 1 ? "schematic" : "schematics"}
              </>
            )}
          </p>
        )}
      </motion.header>

      <motion.div className="biblioteca__barra" variants={variantePieza} custom={direccion}>
        <BarraBiblioteca refBuscar={buscar} alSalirDeBusqueda={salirDeBusqueda} />
      </motion.div>

      <p id={idAyuda} className="solo-lectores">
        Flechas para moverse, Intro para abrir, F2 para renombrar y Supr para enviar a la Papelera.
      </p>

      {/* Sin layoutScroll: medir el desplazamiento en cada uno de los 500 nodos
          costaba más que todo lo demás, y el vuelo compartido cae bien igual
          (el desplazamiento se restaura antes de que Motion mida). */}
      <div ref={cuerpo} className="biblioteca__cuerpo" onKeyDown={alPulsarTecla}>
        {contenido}
      </div>
    </motion.section>
  );
}

interface PropsCabecera {
  campo: CampoOrden;
  etiqueta: string;
  orden: Orden;
  alPulsar: (campo: CampoOrden) => void;
  numerica?: boolean;
}

/** Cabecera de columna que ordena; el clic en la activa invierte el sentido. */
function CabeceraOrdenable({ campo, etiqueta, orden, alPulsar, numerica }: PropsCabecera) {
  const activa = orden.campo === campo;
  const ascendente = orden.direccion === "asc";
  return (
    <th
      scope="col"
      className={numerica ? "lista__num" : undefined}
      aria-sort={activa ? (ascendente ? "ascending" : "descending") : "none"}
    >
      <button type="button" className="lista__ordenar" onClick={() => alPulsar(campo)}>
        {etiqueta}
        <span className="casilla__sentido">
          {activa && <IconoSentido ascendente={ascendente} />}
        </span>
      </button>
    </th>
  );
}
