// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

/** Dimensiones de un schematic en bloques (ejes de Minecraft: X, Y, Z). */
export interface Dimensiones {
  ancho: number;
  alto: number;
  largo: number;
}

export interface EntradaEjemplo {
  id: string;
  nombre: string;
  dimensiones: Dimensiones;
}

/** Entradas de ejemplo hasta que la biblioteca lea archivos reales. */
export const EJEMPLOS: readonly EntradaEjemplo[] = [
  { id: "demo-1", nombre: "Torre de vigía", dimensiones: { ancho: 9, alto: 31, largo: 9 } },
  { id: "demo-2", nombre: "Granja de trigo", dimensiones: { ancho: 24, alto: 7, largo: 16 } },
  { id: "demo-3", nombre: "Puente de piedra", dimensiones: { ancho: 48, alto: 12, largo: 7 } },
];

export function buscarEjemplo(id: string): EntradaEjemplo | undefined {
  return EJEMPLOS.find((e) => e.id === id);
}

/** «9 × 31 × 9»: ancho × alto × largo. */
export function formatoDimensiones({ ancho, alto, largo }: Dimensiones): string {
  return `${ancho} × ${alto} × ${largo}`;
}
