// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef } from "react";
import { motion, useIsPresent } from "motion/react";
import { Bloque } from "../componentes/Bloque";
import { formatoDimensiones, formatoNumero } from "../api/biblioteca";
import { useBiblioteca } from "../estado/biblioteca";
import { useNavegacion } from "../estado/navegacion";
import { muelle } from "../estilos/movimiento";
import { idBloque, idVisor, variantePieza, varianteVista } from "./transicion";
import "./vistas.css";
import "./Biblioteca.css";

export function Biblioteca() {
  const abrir = useNavegacion((s) => s.abrir);
  const direccion = useNavegacion((s) => s.direccion);
  // La vista que sale no debe recibir clics mientras se desvanece.
  const presente = useIsPresent();
  const ultimoId = useNavegacion((s) => s.ultimoId);
  const rejilla = useRef<HTMLUListElement>(null);
  const entradas = useBiblioteca((s) => s.entradas);
  const cargando = useBiblioteca((s) => s.cargando);
  const error = useBiblioteca((s) => s.error);
  const cargar = useBiblioteca((s) => s.cargar);

  useEffect(() => {
    void cargar();
  }, [cargar]);

  // Al volver, el foco regresa a la lámina que se abrió.
  useEffect(() => {
    if (!ultimoId) return;
    rejilla.current
      ?.querySelector<HTMLButtonElement>(`[data-id="${CSS.escape(ultimoId)}"]`)
      ?.focus({ preventScroll: true });
  }, [ultimoId]);

  return (
    <motion.section
      className="vista"
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
        <p className="vista__dato cifras">
          {cargando ? "" : `${formatoNumero(entradas.length)} ${entradas.length === 1 ? "schematic" : "schematics"}`}
        </p>
      </motion.header>

      {error && (
        <p className="biblioteca__mensaje" role="alert">
          No se pudo leer la biblioteca. {error.mensaje}
        </p>
      )}
      {!cargando && !error && entradas.length === 0 && (
        <p className="biblioteca__mensaje">
          Arrastra aquí archivos .litematic, .schem, .schematic o .nbt
        </p>
      )}

      <ul className="biblioteca__rejilla" ref={rejilla}>
        {entradas.map((e) => (
          <motion.li key={e.id} variants={variantePieza} custom={direccion}>
            <button
              type="button"
              className="lamina"
              data-id={e.id}
              onClick={() => abrir(e.id)}
            >
              <motion.div
                className="lamina__visor"
                layoutId={idVisor(e.id)}
                transition={{ layout: muelle("compartido") }}
              >
                <Bloque dimensiones={e.dimensiones} layoutId={idBloque(e.id)} />
              </motion.div>
              <span className="lamina__pie">
                <span className="lamina__nombre">{e.nombre}</span>
                <span className="lamina__medidas cifras" title="Ancho × alto × largo, en bloques">
                  {formatoDimensiones(e.dimensiones)}
                </span>
              </span>
            </button>
          </motion.li>
        ))}
      </ul>
    </motion.section>
  );
}
