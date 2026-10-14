// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Lienzo de Three.js. Va en su propio módulo para que Three, React Three
// Fiber y drei se carguen aparte (import dinámico desde Visor3D) y no
// retrasen el arranque de la biblioteca.

import { Canvas } from "@react-three/fiber";
import { useMemo } from "react";
import type { AtlasCargado } from "../../api/atlas";
import type { Malla } from "../../api/visor";
import { Escena } from "./Escena";
import { atlasVisor } from "./atlasThree";

interface Props {
  id: string;
  malla: Malla;
  /** Atlas decodificado; aquí se convierte en textura de Three. */
  atlas: AtlasCargado | null;
  /** Giro automático alrededor del modelo (tecla Espacio). */
  girando: boolean;
  onListo: () => void;
}

export default function Lienzo({ id, malla, atlas, girando, onListo }: Props) {
  const texturas = useMemo(() => (atlas ? atlasVisor(atlas) : null), [atlas]);
  return (
    <Canvas
      frameloop="demand"
      dpr={[1, 2]}
      flat
      gl={{ antialias: true, powerPreference: "high-performance" }}
      camera={{ fov: 45, near: 0.1, far: 10000, position: [10, 10, 10] }}
      // Medidas sin transformaciones: durante el vuelo del elemento compartido
      // el contenedor va escalado y getBoundingClientRect daría un tamaño falso.
      resize={{ offsetSize: true }}
    >
      <Escena id={id} malla={malla} atlas={texturas} girando={girando} onListo={onListo} />
    </Canvas>
  );
}
