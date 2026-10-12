// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect } from "react";
import { AnimatePresence, LayoutGroup, MotionConfig } from "motion/react";
import { BarraTitulo } from "./componentes/BarraTitulo";
import { useNavegacion } from "./estado/navegacion";
import { muelle } from "./estilos/movimiento";
import { Biblioteca } from "./vistas/Biblioteca";
import { Schematic } from "./vistas/Schematic";
import "./App.css";

export default function App() {
  const ruta = useNavegacion((s) => s.ruta);
  const direccion = useNavegacion((s) => s.direccion);
  const volver = useNavegacion((s) => s.volver);

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
                <Biblioteca key="biblioteca" />
              ) : (
                <Schematic key={`schematic-${ruta.id}`} id={ruta.id} />
              )}
            </AnimatePresence>
          </LayoutGroup>
        </main>
      </div>
    </MotionConfig>
  );
}
