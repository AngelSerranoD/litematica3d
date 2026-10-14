// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

/** Lo mínimo que hace falta saber del elemento con foco para decidir sobre Espacio. */
export interface DatosFoco {
  tagName: string;
  type?: string;
  isContentEditable?: boolean;
  /** Está dentro de un `<dialog>` (o es uno). */
  dentroDeDialogo?: boolean;
}

/**
 * ¿Debe el elemento con foco quedarse con Espacio? Campos de texto, botones,
 * casillas y demás controles lo usan para lo suyo, y dentro de un diálogo el
 * teclado es del diálogo. Devuelve el motivo, o `null` si el atajo puede usarse.
 */
export function espacioBloqueadoPor(el: DatosFoco | null): string | null {
  if (!el) return null;
  if (el.dentroDeDialogo) return "dialogo";
  if (el.isContentEditable) return "editable";
  const etiqueta = el.tagName.toUpperCase();
  if (etiqueta === "INPUT" || etiqueta === "TEXTAREA" || etiqueta === "SELECT") return "campo";
  if (etiqueta === "BUTTON" || etiqueta === "SUMMARY" || etiqueta === "A") return "boton";
  return null;
}

/** Extrae los datos del elemento con foco del DOM (y mira si hay un diálogo abierto). */
export function datosDeFoco(el: Element | null, doc: Document = document): DatosFoco | null {
  if (!el || el === doc.body) return doc.querySelector("dialog[open]") ? { tagName: "DIALOG", dentroDeDialogo: true } : null;
  return {
    tagName: el.tagName,
    type: el instanceof HTMLInputElement ? el.type : undefined,
    isContentEditable: el instanceof HTMLElement && el.isContentEditable,
    dentroDeDialogo: el.closest("dialog") !== null || doc.querySelector("dialog[open]") !== null,
  };
}
