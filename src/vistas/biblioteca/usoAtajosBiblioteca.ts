// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Atajos de teclado globales mientras se ve la biblioteca.

import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import { ultimoPendiente } from "../../componentes/Pendientes";
import { useAjustes } from "../../estado/ajustes";
import { useBiblioteca } from "../../estado/biblioteca";
import { useFoco } from "../../estado/foco";

/** El objetivo de la tecla es un campo de texto (ahí las teclas son suyas). */
export const esEditable = (el: EventTarget | null) =>
  el instanceof HTMLElement &&
  (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable);

/**
 * Ctrl+O importa, Ctrl+F o «/» van al buscador y Ctrl+Z deshace el último
 * borrado. Con la vista saliendo (`presente` falso) o los ajustes abiertos,
 * no hacen nada.
 */
export function usoAtajosBiblioteca(
  buscar: RefObject<HTMLInputElement | null>,
  presente: boolean,
): void {
  // Lo último de cada render, para el oyente (que se registra una vez).
  const presenteActual = useRef(presente);
  useLayoutEffect(() => {
    presenteActual.current = presente;
  });

  useEffect(() => {
    const alPulsar = (e: KeyboardEvent) => {
      if (!presenteActual.current || e.defaultPrevented || useAjustes.getState().abierto) return;
      const tecla = e.key.toLowerCase();
      const ctrl = e.ctrlKey || e.metaKey;

      if (ctrl && !e.shiftKey && !e.altKey && tecla === "o") {
        e.preventDefault();
        void useBiblioteca.getState().elegirEImportar();
      } else if (ctrl && !e.shiftKey && !e.altKey && tecla === "f") {
        e.preventDefault();
        buscar.current?.focus();
        buscar.current?.select();
      } else if (e.key === "/" && !ctrl && !e.altKey && !esEditable(e.target)) {
        e.preventDefault();
        buscar.current?.focus();
        buscar.current?.select();
      } else if (ctrl && !e.shiftKey && !e.altKey && tecla === "z" && !esEditable(e.target)) {
        const pendiente = ultimoPendiente(useBiblioteca.getState().pendientesDeBorrar);
        if (!pendiente) return;
        e.preventDefault();
        useBiblioteca.getState().deshacerEliminar(pendiente.entrada.id);
        useFoco.getState().pedir(pendiente.entrada.id);
      }
    };
    window.addEventListener("keydown", alPulsar);
    return () => window.removeEventListener("keydown", alPulsar);
  }, [buscar]);
}
