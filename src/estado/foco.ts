// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { create } from "zustand";

/**
 * Petición de foco para un schematic de la biblioteca.
 *
 * Sirve cuando quien actúa no es la propia biblioteca: el botón «Deshacer»
 * del aviso de borrado devuelve la entrada a su sitio y, como el botón
 * desaparece, el foco tiene que ir a la entrada recuperada (si no, se
 * quedaría en el <body>). La biblioteca atiende la petición en cuanto la
 * entrada vuelve a estar en el DOM.
 */
interface EstadoFoco {
  /** `n` cambia en cada petición, aunque se repita el id. */
  pedido: { id: string; n: number } | null;
  pedir: (id: string) => void;
  atender: () => void;
}

export const useFoco = create<EstadoFoco>()((set) => ({
  pedido: null,
  pedir: (id) => set((s) => ({ pedido: { id, n: (s.pedido?.n ?? 0) + 1 } })),
  atender: () => set({ pedido: null }),
}));
