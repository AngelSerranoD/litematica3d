// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { AnimatePresence, LayoutGroup, LazyMotion, MotionConfig } from "motion/react";
import { Avisos, ZonaSoltar } from "./componentes/Avisos";
import { SaltarAlContenido } from "./componentes/SaltarAlContenido";
import { BarraTitulo } from "./componentes/BarraTitulo";
import { PanelAjustes } from "./componentes/PanelAjustes";
import { useAjustes } from "./estado/ajustes";
import { useBiblioteca } from "./estado/biblioteca";
import { iniciarMiniaturas } from "./estado/miniaturas";
import { useNavegacion } from "./estado/navegacion";
import { usoAbrirArchivo } from "./estado/usoAbrirArchivo";
import { usoCierreVentana } from "./estado/usoCierreVentana";
import { usoSoltarArchivos } from "./estado/usoSoltarArchivos";
import { muelle } from "./estilos/movimiento";
import { Biblioteca } from "./vistas/Biblioteca";
import { Schematic } from "./vistas/Schematic";
import "./App.css";

// Carga diferida de las funciones de Motion: fuera del chunk de entrada.
const cargarFunciones = () => import("./estilos/funcionesMotion").then((r) => r.default);

export default function App() {
  const ruta = useNavegacion((s) => s.ruta);
  const direccion = useNavegacion((s) => s.direccion);
  const volver = useNavegacion((s) => s.volver);
  const soltando = usoSoltarArchivos();
  // Con la biblioteca vacía, la propia vista es la zona de soltar: no se
  // superpone la capa general.
  const vacia = useBiblioteca((s) => !s.cargando && !s.error && s.entradas.length === 0);
  usoCierreVentana();
  usoAbrirArchivo();

  // Ajustes guardados (tema, fluidos…) desde el arranque.
  useEffect(() => {
    void useAjustes.getState().iniciar();
  }, []);

  // Miniaturas isométricas en segundo plano: las que falten o estén
  // obsoletas, y todas otra vez al cambiar las texturas.
  useEffect(() => iniciarMiniaturas(), []);

  // Esc vuelve a la biblioteca desde cualquier punto del schematic (salvo con
  // los ajustes abiertos: ahí Esc solo cierra el diálogo). Ctrl+, abre los
  // ajustes desde cualquier vista.
  useEffect(() => {
    const alPulsar = (e: KeyboardEvent) => {
      if (e.defaultPrevented) return;
      const ajustes = useAjustes.getState();
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key === ",") {
        e.preventDefault();
        void ajustes.abrir();
      } else if (e.key === "Escape" && !ajustes.abierto) {
        volver();
      }
    };
    window.addEventListener("keydown", alPulsar);
    return () => window.removeEventListener("keydown", alPulsar);
  }, [volver]);

  return (
    // reducedMotion="user": con «reducir movimiento» del sistema, Motion
    // anula transformaciones y vuelos de layout y deja solo la opacidad.
    <LazyMotion features={cargarFunciones} strict>
      <MotionConfig reducedMotion="user" transition={muelle("vista")}>
        <div className="app">
          <SaltarAlContenido />
          <BarraTitulo />
          <main className="app__vistas papel">
            <LayoutGroup>
              <AnimatePresence initial={false} custom={direccion}>
                {ruta.vista === "biblioteca" ? (
                  <Biblioteca key="biblioteca" soltando={soltando} />
                ) : (
                  <Schematic key={`schematic-${ruta.id}`} id={ruta.id} />
                )}
              </AnimatePresence>
            </LayoutGroup>
            <ZonaSoltar visible={soltando && !(vacia && ruta.vista === "biblioteca")} />
            <Avisos />
            <PanelAjustes />
          </main>
        </div>
      </MotionConfig>
    </LazyMotion>
  );
}
