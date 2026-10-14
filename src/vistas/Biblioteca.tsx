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
  type ReactNode,
} from "react";
import { flushSync } from "react-dom";
import { m, useIsPresent } from "motion/react";
import { BarraBiblioteca } from "../componentes/BarraBiblioteca";
import { BibliotecaVacia } from "../componentes/BibliotecaVacia";
import { Cifra } from "../componentes/Cifra";
import type { AccionesElemento, InfoEntrada } from "../logica/elementoBiblioteca";
import { useBiblioteca, useEntradasVisibles } from "../estado/biblioteca";
import { useNavegacion } from "../estado/navegacion";
import { ListaBiblioteca } from "./biblioteca/ListaBiblioteca";
import { RejillaBiblioteca } from "./biblioteca/RejillaBiblioteca";
import { usoAtajosBiblioteca } from "./biblioteca/usoAtajosBiblioteca";
import { usoSeleccion } from "./biblioteca/usoSeleccion";
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
  const ultimaImportada = useBiblioteca((s) => s.ultimaImportada);
  const cargar = useBiblioteca((s) => s.cargar);
  const establecerBusqueda = useBiblioteca((s) => s.establecerBusqueda);

  const cuerpo = useRef<HTMLDivElement>(null);
  const contenedor = useRef<HTMLUListElement & HTMLTableElement>(null);
  const buscar = useRef<HTMLInputElement>(null);
  const idAyuda = useId();

  // El schematic del que se vuelve (si se vuelve de uno): llega volando a
  // su lámina, así que esa lámina no hace la entrada escalonada.
  const [idFijo] = useState(() => (direccion === -1 ? useNavegacion.getState().ultimoId : null));
  const [renombrandoId, setRenombrandoId] = useState<string | null>(null);
  // Elemento con layoutId compartido con la vista Schematic (ver Lamina).
  const [compartidoId, setCompartidoId] = useState<string | null>(idFijo);

  const { activar, botonDe, enfocar, enfocarActivo, eliminar, alPulsarTecla } = usoSeleccion({
    cuerpo,
    contenedor,
    visibles,
    activoInicial: idFijo,
    alRenombrar: setRenombrandoId,
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

  // Al volver de un schematic: mismo desplazamiento y foco en su lámina. En
  // un efecto de layout para que Motion mida el destino del vuelo ya
  // desplazado.
  useLayoutEffect(() => {
    if (!idFijo || !cuerpo.current) return;
    cuerpo.current.scrollTop = desplazamientoGuardado;
    botonDe(idFijo)?.focus({ preventScroll: true });
  }, [idFijo, botonDe]);

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
      activar,
      empezarRenombrado: (id) => setRenombrandoId(id),
      terminarRenombrado: (id, nombre, devolverFoco) => {
        setRenombrandoId(null);
        if (nombre) void useBiblioteca.getState().renombrar(id, nombre);
        if (devolverFoco) enfocar(id);
      },
      eliminar,
    }),
    [abrirSchematic, activar, enfocar, eliminar],
  );

  usoAtajosBiblioteca(buscar, presente);

  const salirDeBusqueda = useCallback(() => {
    if (!enfocarActivo()) buscar.current?.blur();
  }, [enfocarActivo]);

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
  } else {
    const Coleccion = modoVista === "rejilla" ? RejillaBiblioteca : ListaBiblioteca;
    contenido = (
      <Coleccion
        refContenedor={contenedor}
        visibles={visibles}
        renombrandoId={renombrandoId}
        compartidoId={compartidoId}
        infoEntrada={infoEntrada}
        acciones={acciones}
        idAyuda={idAyuda}
      />
    );
  }

  return (
    <m.section
      className="vista vista--biblioteca"
      aria-labelledby="titulo-biblioteca"
      variants={varianteVista}
      initial="oculta"
      animate="visible"
      exit="saliendo"
      custom={direccion}
      inert={!presente}
    >
      <m.header className="vista__cabecera" variants={variantePieza} custom={direccion}>
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
      </m.header>

      <m.div className="biblioteca__barra" variants={variantePieza} custom={direccion}>
        <BarraBiblioteca refBuscar={buscar} alSalirDeBusqueda={salirDeBusqueda} />
      </m.div>

      <p id={idAyuda} className="solo-lectores">
        Flechas para moverse, Intro para abrir, F2 para renombrar y Supr para enviar a la Papelera.
      </p>

      {/* Sin layoutScroll: medir el desplazamiento en cada uno de los 500 nodos
          costaba más que todo lo demás, y el vuelo compartido cae bien igual
          (el desplazamiento se restaura antes de que Motion mida). */}
      <div
        ref={cuerpo}
        id="contenido-principal"
        className="biblioteca__cuerpo"
        tabIndex={-1}
        onKeyDown={alPulsarTecla}
      >
        {contenido}
      </div>
    </m.section>
  );
}
