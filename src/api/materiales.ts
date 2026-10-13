// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Comandos de la pantalla de materiales: lista de comprobación, nombres de
// los ítems (en inglés) y guardado del CSV. El cálculo en sí es `materiales`
// (src/api/biblioteca.ts).

import { llamar } from "./llamar";

/** IDs de ítem (`minecraft:…`) marcados como reunidos en este schematic. */
export const comprobacion = (id: string) => llamar<string[]>("comprobacion", { id });

/** Marca o desmarca un ítem como reunido. */
export const marcar = (id: string, item: string, reunido: boolean) =>
  llamar<null>("marcar", { id, item, reunido });

/**
 * Nombres en inglés de los ítems (`minecraft:stone` → «Stone»), sacados
 * del jar de Minecraft. Vacío si no hay Minecraft instalado.
 */
export const nombresItems = () => llamar<Record<string, string>>("nombres_items");

/**
 * Pide con el diálogo de guardar dónde dejar el CSV y lo escribe. `false` si
 * el usuario cancela.
 */
export const guardarCsv = (nombre: string, contenido: string) =>
  llamar<boolean>("guardar_csv", { nombre, contenido });
