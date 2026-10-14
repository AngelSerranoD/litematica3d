// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Bloques y familias parecidos (docs/edicion.md, «Parecidos»): sugerir un
// bloque de la misma forma y color parecido, saber a qué familia de material
// pertenece un bloque y preparar el cambio de una familia entera por otra.
// Los planes se aplican con `aplicarCambios(id, [], plan)`.

import { llamar } from "./llamar";
import type { Reemplazo } from "./edicion";

/** Forma de un bloque: solo se sugieren bloques de la misma forma. */
type FormaBloque =
  | "entero"
  | "losa"
  | "escalera"
  | "muro"
  | "valla"
  | "puertaValla"
  | "puerta"
  | "trampilla"
  | "boton"
  | "placaPresion"
  | "panel"
  | "alfombra"
  | "otro";

/** Color sRGB de 8 bits. */
export type Rgb = [number, number, number];

/** Un bloque parecido, listo para usar como destino de un cambio. */
export interface Parecido {
  /** `minecraft:polished_blackstone_brick_stairs`. */
  id: string;
  /** Block state destino con las propiedades del origen que admite (facing, half…). */
  estado: string;
  forma: FormaBloque;
  familia: string;
  color: Rgb;
  /** Distancia OKLab al color del origen (menos de ~0,02 apenas se nota). */
  distancia: number;
}

/** Familia de material y variante de un bloque. */
interface FamiliaBloque {
  /** `deepslate`, `oak`, `quartz`… */
  familia: string;
  /** Componentes de la variante (`["pulido", "ladrillo"]`); vacío = la base. */
  variante: string[];
  /** `base` o los componentes unidos con `+` (`pulido+ladrillo`). */
  nombreVariante: string;
  forma: FormaBloque;
  /** ¿Se puede sugerir como destino? (no lo son el aire, los cofres…). */
  sugerible: boolean;
}

/** Un bloque del catálogo dentro de su familia. */
interface Miembro {
  id: string;
  forma: FormaBloque;
  variante: string[];
  sugerible: boolean;
}

/** Una familia parecida a otra. */
export interface FamiliaParecida {
  familia: string;
  /** Color medio de sus miembros. */
  color: Rgb;
  /** Distancia OKLab entre los colores medios. */
  distancia: number;
  /** Formas de la familia origen que también tiene esta. */
  formasComunes: number;
  /** De 0 a 1: lo equivalentes que son sus variantes a las de la origen. */
  variantesComunes: number;
  /** Orden de la lista (menor = más parecida). */
  puntuacion: number;
  /** Sus miembros sugeribles. */
  miembros: Miembro[];
}

/** Un block state presente en el schematic y cuántas posiciones ocupa. */
export interface EstadoPresente {
  estado: string;
  cantidad: number;
}

/**
 * Los `n` bloques sugeribles de la misma forma más parecidos en color
 * (12 si no se dice; máximo 100), del más al menos parecido.
 */
export const parecidos = (estado: string, n?: number) =>
  llamar<Parecido[]>("parecidos", { estado, n });

/** Familia, variante y forma de un block state (el de `bloque_en`). */
export const familiaDe = (estado: string) => llamar<FamiliaBloque>("familia_de", { estado });

/** Block states de una familia presentes en el schematic, ordenados por estado. */
export const miembrosFamilia = (id: string, familia: string) =>
  llamar<EstadoPresente[]>("miembros_familia", { id, familia });

/** Familias más parecidas a una (12 si no se dice; máximo 100). */
export const familiasParecidas = (familia: string, n?: number) =>
  llamar<FamiliaParecida[]>("familias_parecidas", { familia, n });

/**
 * Reemplazos para cambiar la familia `familiaOrigen` por `familiaDestino` en
 * el schematic. Se aplican tal cual con `aplicarCambios(id, [], plan)`.
 */
export const planFamilia = (id: string, familiaOrigen: string, familiaDestino: string) =>
  llamar<Reemplazo[]>("plan_familia", { id, familiaOrigen, familiaDestino });

/**
 * Reemplazo de cada estado de `estadosOrigen` por el bloque de `estadoDestino`
 * conservando las propiedades de cada uno (facing, half, shape…); lo resuelve
 * Rust con la misma regla que las sugerencias.
 */
export const destinosConservando = (estadoDestino: string, estadosOrigen: string[]) =>
  llamar<Reemplazo[]>("destinos_conservando", { estadoDestino, estadosOrigen });
