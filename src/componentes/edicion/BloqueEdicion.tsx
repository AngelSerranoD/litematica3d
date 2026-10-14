// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Nombre e icono de un block state, comunes a todo el panel de edición.

import { useEffect, type CSSProperties } from "react";
import type { Rgb } from "../../api/parecidos";
import { celdaIcono, useEdicion } from "../../estado/edicion";
import { useMateriales } from "../../estado/materiales";
import { nombreItem } from "../../logica/materiales";

/** `minecraft:oak_stairs[facing=north]` → `minecraft:oak_stairs`. */
const idDe = (estado: string) => estado.split("[", 1)[0];

/** Nombre en inglés del bloque de un block state (como en los materiales). */
export function useNombre(estado: string): string {
  const nombres = useMateriales((s) => s.nombres);
  return nombreItem(idDe(estado), nombres);
}

/**
 * Icono de un block state: su textura del atlas (por `celdas_atlas`, aunque
 * no esté en el schematic) o, sin atlas, su color medio.
 */
export function IconoBloque({
  estado,
  color,
  lado = 24,
}: {
  estado: string;
  color?: Rgb | null;
  /** Lado en píxeles. */
  lado?: number;
}) {
  const fila = useEdicion((s) => s.celdas[estado]);
  const versionCeldas = useEdicion((s) => s.versionCeldas);
  const atlas = useMateriales((s) => s.modelo?.atlas ?? null);
  // Los estados que no vinieron en una lista de parecidos (p. ej. los de un
  // plan de familia) se piden aquí; el almacén junta las peticiones.
  useEffect(() => {
    if (fila === undefined) useEdicion.getState().pedirCeldas([estado]);
  }, [estado, fila]);
  const celda = celdaIcono(fila);
  if (celda && atlas && atlas.datos.version === versionCeldas) {
    const d = atlas.datos;
    // k escala el atlas entero para que una textura mida `lado`.
    const k = lado / d.tamTextura;
    const x = (celda.celda % d.celdasPorFila) * d.tamCelda + d.relleno;
    const y = Math.floor(celda.celda / d.celdasPorFila) * d.tamCelda + d.relleno;
    const estilo: CSSProperties = {
      width: lado,
      height: lado,
      backgroundImage: `url("${atlas.url}")`,
      backgroundSize: `${d.ancho * k}px ${d.alto * k}px`,
      backgroundPosition: `${-x * k}px ${-y * k}px`,
    };
    return (
      <span className="icono-bloque icono-bloque--textura" style={estilo} aria-hidden="true" />
    );
  }
  const c = celda?.color ?? color ?? null;
  return (
    <span
      className={c ? "icono-bloque" : "icono-bloque icono-bloque--neutro"}
      style={{
        width: lado,
        height: lado,
        backgroundColor: c ? `rgb(${c[0]} ${c[1]} ${c[2]})` : undefined,
      }}
      aria-hidden="true"
    />
  );
}

/** Nombre del bloque con su icono delante. */
export function Bloque({ estado, color }: { estado: string; color?: Rgb | null }) {
  const nombre = useNombre(estado);
  return (
    <span className="bloque-edicion">
      <IconoBloque estado={estado} color={color} lado={16} />
      <span className="bloque-edicion__nombre" title={estado}>
        {nombre}
      </span>
    </span>
  );
}
