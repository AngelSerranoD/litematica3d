// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { m, useIsPresent } from "motion/react";
import { Cifra } from "../componentes/Cifra";
import { Miniatura } from "../componentes/Miniatura";
import { ConfirmacionEdicion } from "../componentes/edicion/ConfirmacionEdicion";
import { PanelEdicion } from "../componentes/edicion/PanelEdicion";
import { PanelMateriales } from "../componentes/materiales/PanelMateriales";
import { Visor3D } from "../componentes/visor/Visor3D";
import { obtener, type Region } from "../api/biblioteca";
import { formatoDimensiones, nombreFormato } from "../logica/formato";
import { useBiblioteca } from "../estado/biblioteca";
import { useEdicion } from "../estado/edicion";
import { fijarGuardaSalida, useNavegacion } from "../estado/navegacion";
import { muelle } from "../estilos/movimiento";
import { idBloque, idVisor } from "../estilos/idsTransicion";
import { variantePieza, varianteVista } from "./transicion";
import "./vistas.css";
import "./Schematic.css";

type Pestana = "ficha" | "materiales";

const PESTANAS: { valor: Pestana; etiqueta: string }[] = [
  { valor: "ficha", etiqueta: "Ficha" },
  { valor: "materiales", etiqueta: "Materiales" },
];

const CLAVE_PESTANA = "schematiclab.pestanaSchematic";

/** Pestaña recordada entre schematics (comodidad: si falla, la ficha). */
function pestanaGuardada(): Pestana {
  try {
    return localStorage.getItem(CLAVE_PESTANA) === "materiales" ? "materiales" : "ficha";
  } catch {
    return "ficha";
  }
}

export function Schematic({ id }: { id: string }) {
  const volver = useNavegacion((s) => s.volver);
  const direccion = useNavegacion((s) => s.direccion);
  // La vista que sale no debe recibir clics mientras se desvanece.
  const presente = useIsPresent();
  const entrada = useBiblioteca((s) => s.entradas.find((e) => e.id === id));
  const cargando = useBiblioteca((s) => s.cargando);
  const [regiones, setRegiones] = useState<Region[]>([]);
  const botonVolver = useRef<HTMLButtonElement>(null);
  const [pestana, setPestana] = useState<Pestana>(pestanaGuardada);
  const modo = useEdicion((s) => s.modo);
  const refPestanas = useRef<Record<Pestana, HTMLButtonElement | null>>({
    ficha: null,
    materiales: null,
  });

  const elegirPestana = (p: Pestana) => {
    setPestana(p);
    try {
      localStorage.setItem(CLAVE_PESTANA, p);
    } catch {
      // No es grave: solo se pierde la preferencia.
    }
  };

  // Patrón de pestañas: las flechas cambian de pestaña y llevan el foco.
  const alPulsarPestanas = (e: KeyboardEvent<HTMLDivElement>) => {
    const i = PESTANAS.findIndex((p) => p.valor === pestana);
    let j: number;
    if (e.key === "ArrowRight") j = (i + 1) % PESTANAS.length;
    else if (e.key === "ArrowLeft") j = (i - 1 + PESTANAS.length) % PESTANAS.length;
    else if (e.key === "Home") j = 0;
    else if (e.key === "End") j = PESTANAS.length - 1;
    else return;
    e.preventDefault();
    const destino = PESTANAS[j].valor;
    elegirPestana(destino);
    refPestanas.current[destino]?.focus();
  };

  useEffect(() => {
    botonVolver.current?.focus({ preventScroll: true });
  }, []);

  // Edición: el estado es de este schematic, y salir con cambios pendientes
  // (Volver, Esc, abrir otro) pregunta antes si se descartan.
  useEffect(() => {
    const edicion = useEdicion.getState();
    edicion.iniciar(id);
    const quitarGuarda = fijarGuardaSalida((continuar) =>
      useEdicion.getState().pedirDescartar("salirSchematic", continuar),
    );
    return () => {
      quitarGuarda();
      // Con AnimatePresence el siguiente schematic puede montarse antes de que
      // este se desmonte: solo se limpia si el estado sigue siendo de este.
      if (useEdicion.getState().id === id) useEdicion.getState().iniciar(null);
    };
  }, [id]);

  // Las regiones no van en la lista: se piden al abrir la ficha.
  useEffect(() => {
    let activo = true;
    obtener(id)
      .then((d) => activo && setRegiones(d.regiones))
      .catch(() => activo && setRegiones([]));
    return () => {
      activo = false;
    };
  }, [id]);

  const dim = entrada?.dimensiones;

  return (
    <m.section
      className="vista"
      aria-labelledby="titulo-schematic"
      variants={varianteVista}
      initial="oculta"
      animate="visible"
      exit="saliendo"
      custom={direccion}
      inert={!presente}
    >
      <m.header
        className="vista__cabecera vista__cabecera--schematic"
        variants={variantePieza}
        custom={direccion}
      >
        <button type="button" className="boton-plano" ref={botonVolver} onClick={volver}>
          <svg viewBox="0 0 12 10" width="12" height="10" aria-hidden="true">
            <path d="M5 0.5L0.5 5L5 9.5M0.5 5H12" />
          </svg>
          Volver
          <kbd>Esc</kbd>
        </button>
        <h1 id="titulo-schematic" className="vista__titulo vista__titulo--schematic rotulo">
          {entrada?.nombre ?? (cargando ? "" : "No encontrado")}
        </h1>
        {entrada && <p className="vista__dato cifras">{nombreFormato(entrada.formato)}</p>}
      </m.header>

      <div
        className={
          modo === "modificar"
            ? "schematic__cuerpo schematic__cuerpo--edicion"
            : pestana === "materiales"
              ? "schematic__cuerpo schematic__cuerpo--materiales"
              : "schematic__cuerpo"
        }
      >
        <div className="schematic__marco">
          <m.div
            id="contenido-principal"
            className="schematic__visor"
            tabIndex={-1}
            layoutId={idVisor(id)}
            transition={{ layout: muelle("compartido") }}
          >
            {dim && (
              <Visor3D
                id={id}
                marcador={
                  <Miniatura
                    id={id}
                    dimensiones={dim}
                    miniatura={entrada?.miniatura}
                    layoutId={idBloque(id)}
                  />
                }
              />
            )}
            {!entrada && !cargando && (
              <p className="schematic__nota schematic__nota--centro" role="alert">
                Este schematic ya no está en la biblioteca.
              </p>
            )}
          </m.div>
        </div>

        <m.aside
          className="schematic__ficha"
          aria-label={modo === "modificar" ? "Edición del schematic" : "Datos del schematic"}
          variants={variantePieza}
          custom={direccion}
        >
          {modo === "modificar" ? (
            <PanelEdicion />
          ) : (
            <>
              <div
                className="schematic__pestanas"
                role="tablist"
                aria-label="Datos del schematic"
                onKeyDown={alPulsarPestanas}
              >
                {PESTANAS.map(({ valor, etiqueta }) => (
                  <button
                    key={valor}
                    ref={(el) => {
                      refPestanas.current[valor] = el;
                    }}
                    type="button"
                    role="tab"
                    id={`pestana-${valor}`}
                    className="schematic__pestana rotulo"
                    aria-selected={pestana === valor}
                    aria-controls={pestana === valor ? `panel-${valor}` : undefined}
                    tabIndex={pestana === valor ? 0 : -1}
                    onClick={() => elegirPestana(valor)}
                  >
                    {etiqueta}
                  </button>
                ))}
              </div>

              {pestana === "materiales" ? (
                <div
                  className="schematic__panel schematic__panel--materiales"
                  role="tabpanel"
                  id="panel-materiales"
                  aria-labelledby="pestana-materiales"
                >
                  <PanelMateriales
                    id={id}
                    titulo={entrada?.nombre ?? "schematic"}
                    regiones={regiones}
                  />
                </div>
              ) : (
                <div
                  className="schematic__panel"
                  role="tabpanel"
                  id="panel-ficha"
                  aria-labelledby="pestana-ficha"
                >
                  <dl>
                    {entrada && (
                      <>
                        <div className="ficha__fila">
                          <dt>Formato</dt>
                          <dd>{nombreFormato(entrada.formato)}</dd>
                        </div>
                        <div className="ficha__fila">
                          <dt>Bloques</dt>
                          <dd><Cifra valor={entrada.totalBloques} /></dd>
                        </div>
                        <div className="ficha__fila">
                          <dt>Regiones</dt>
                          <dd><Cifra valor={entrada.regiones} /></dd>
                        </div>
                      </>
                    )}
                    {dim && (
                      <>
                        <div className="ficha__fila">
                          <dt>Medidas</dt>
                          <dd className="cifras">{formatoDimensiones(dim)}</dd>
                        </div>
                        <div className="ficha__fila ficha__fila--sub">
                          <dt>Ancho</dt>
                          <dd className="cifras">{dim.ancho}</dd>
                        </div>
                        <div className="ficha__fila ficha__fila--sub">
                          <dt>Alto</dt>
                          <dd className="cifras">{dim.alto}</dd>
                        </div>
                        <div className="ficha__fila ficha__fila--sub">
                          <dt>Largo</dt>
                          <dd className="cifras">{dim.largo}</dd>
                        </div>
                        <div className="ficha__fila">
                          <dt>Volumen</dt>
                          <dd>
                            <Cifra valor={dim.ancho * dim.alto * dim.largo} /> bloques
                          </dd>
                        </div>
                      </>
                    )}
                  </dl>
                  {regiones.length > 1 && (
                    <>
                      <h2 className="ficha__subtitulo rotulo">Regiones</h2>
                      <ul className="ficha__regiones">
                        {regiones.map((g) => (
                          <li key={g.nombre} className="ficha__fila">
                            <span>{g.nombre}</span>
                            <Cifra valor={g.totalBloques} />
                          </li>
                        ))}
                      </ul>
                    </>
                  )}
                </div>
              )}
            </>
          )}
        </m.aside>
      </div>

      <ConfirmacionEdicion />
    </m.section>
  );
}
