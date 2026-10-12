// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef } from "react";
import { motion, useIsPresent } from "motion/react";
import { Bloque } from "../componentes/Bloque";
import { buscarEjemplo, formatoDimensiones } from "../datos/ejemplos";
import { useNavegacion } from "../estado/navegacion";
import { muelle } from "../estilos/movimiento";
import { idBloque, idVisor, variantePieza, varianteVista } from "./transicion";
import "./vistas.css";
import "./Schematic.css";

const numero = new Intl.NumberFormat("es-ES");

export function Schematic({ id }: { id: string }) {
  const volver = useNavegacion((s) => s.volver);
  const direccion = useNavegacion((s) => s.direccion);
  // La vista que sale no debe recibir clics mientras se desvanece.
  const presente = useIsPresent();
  const entrada = buscarEjemplo(id);
  const botonVolver = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    botonVolver.current?.focus({ preventScroll: true });
  }, []);

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
          {entrada?.nombre ?? "Schematic"}
        </h1>
        <p className="vista__dato cifras">{id}</p>
      </motion.header>

      <div className="schematic__cuerpo">
        <div className="schematic__marco">
          <motion.div
            className="schematic__visor"
            layoutId={idVisor(id)}
            transition={{ layout: muelle("compartido") }}
          >
            {dim && <Bloque dimensiones={dim} layoutId={idBloque(id)} />}
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
            <div className="ficha__fila">
              <dt>Identificador</dt>
              <dd className="cifras">{id}</dd>
            </div>
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
                    {numero.format(dim.ancho * dim.alto * dim.largo)} bloques
                  </dd>
                </div>
              </>
            )}
          </dl>
        </motion.aside>
      </div>
    </motion.section>
  );
}
