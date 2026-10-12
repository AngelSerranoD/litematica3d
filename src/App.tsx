// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig } from "motion/react";
import { Avisos, ZonaSoltar } from "./componentes/Avisos";
import { BarraTitulo } from "./componentes/BarraTitulo";
import { PanelAjustes } from "./componentes/PanelAjustes";
import { useBiblioteca } from "./estado/biblioteca";
import { useNavegacion } from "./estado/navegacion";
import { usoCierreVentana } from "./estado/usoCierreVentana";
import { usoSoltarArchivos } from "./estado/usoSoltarArchivos";
import { muelle } from "./estilos/movimiento";
import { Biblioteca } from "./vistas/Biblioteca";
import { Schematic } from "./vistas/Schematic";
import "./App.css";

export default function App() {
  const ruta = useNavegacion((s) => s.ruta);
  const direccion = useNavegacion((s) => s.direccion);
  const volver = useNavegacion((s) => s.volver);
  const soltando = usoSoltarArchivos();
  // Con la biblioteca vacía, la propia vista es la zona de soltar: no se
  // superpone la capa general.
  const vacia = useBiblioteca((s) => !s.cargando && !s.error && s.entradas.length === 0);
  usoCierreVentana();

  // Esc vuelve a la biblioteca desde cualquier punto del schematic.
  useEffect(() => {
    const alPulsar = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !e.defaultPrevented) volver();
    };
    window.addEventListener("keydown", alPulsar);
    return () => window.removeEventListener("keydown", alPulsar);
  }, [volver]);

  return (
    // reducedMotion="user": con «reducir movimiento» del sistema, Motion
    // anula transformaciones y vuelos de layout y deja solo la opacidad.
    <MotionConfig reducedMotion="user" transition={muelle("vista")}>
      <div className="app">
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
  );
}
