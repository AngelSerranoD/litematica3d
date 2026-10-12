// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef, useState } from "react";
import { motion, useIsPresent } from "motion/react";
import { Miniatura } from "../componentes/Miniatura";
import {
  formatoDimensiones,
  formatoNumero,
  nombreFormato,
  obtener,
  type Region,
} from "../api/biblioteca";
import { useBiblioteca } from "../estado/biblioteca";
import { useNavegacion } from "../estado/navegacion";
import { muelle } from "../estilos/movimiento";
import { idBloque, idVisor, variantePieza, varianteVista } from "./transicion";
import "./vistas.css";
import "./Schematic.css";

export function Schematic({ id }: { id: string }) {
  const volver = useNavegacion((s) => s.volver);
  const direccion = useNavegacion((s) => s.direccion);
  // La vista que sale no debe recibir clics mientras se desvanece.
  const presente = useIsPresent();
  const entrada = useBiblioteca((s) => s.entradas.find((e) => e.id === id));
  const cargando = useBiblioteca((s) => s.cargando);
  const [regiones, setRegiones] = useState<Region[]>([]);
  const botonVolver = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    botonVolver.current?.focus({ preventScroll: true });
  }, []);

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
    <motion.section
      className="vista"
      aria-labelledby="titulo-schematic"
      variants={varianteVista}
      initial="oculta"
      animate="visible"
      exit="saliendo"
      custom={direccion}
      inert={!presente}
    >
      <motion.header
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
      </motion.header>

      <div className="schematic__cuerpo">
        <div className="schematic__marco">
          <motion.div
            className="schematic__visor"
            layoutId={idVisor(id)}
            transition={{ layout: muelle("compartido") }}
          >
            {dim && <Miniatura id={id} dimensiones={dim} layoutId={idBloque(id)} />}
            {!entrada && !cargando && (
              <p className="schematic__nota schematic__nota--centro" role="alert">
                Este schematic ya no está en la biblioteca.
              </p>
            )}
          </motion.div>
          <motion.p className="schematic__nota" variants={variantePieza} custom={direccion}>
            Vista previa plana. El modelo en 3D se verá aquí.
          </motion.p>
        </div>

        <motion.aside
          className="schematic__ficha"
          aria-label="Ficha del schematic"
          variants={variantePieza}
          custom={direccion}
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
                  <dd className="cifras">{formatoNumero(entrada.totalBloques)}</dd>
                </div>
                <div className="ficha__fila">
                  <dt>Regiones</dt>
                  <dd className="cifras">{formatoNumero(entrada.regiones)}</dd>
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
                  <dd className="cifras">
                    {formatoNumero(dim.ancho * dim.alto * dim.largo)} bloques
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
                    <span className="cifras">{formatoNumero(g.totalBloques)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </motion.aside>
      </div>
    </motion.section>
  );
}
