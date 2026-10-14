// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { EXTENSIONES_ADMITIDAS } from "../api/dialogo";
import { useBiblioteca } from "./biblioteca";

const EXTENSIONES = EXTENSIONES_ADMITIDAS.map((ext) => `.${ext}`);

const admitido = (ruta: string) => {
  const minusculas = ruta.toLowerCase();
  return EXTENSIONES.some((ext) => minusculas.endsWith(ext));
};

const nombreDe = (ruta: string) => ruta.split(/[\\/]/).pop() ?? ruta;

/**
 * Escucha el arrastrar y soltar de la ventana de Tauri. Devuelve `true`
 * mientras hay archivos sobre la ventana (para la superposición).
 */
export function usoSoltarArchivos(): boolean {
  const [soltando, setSoltando] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    let activo = true;
    let soltar: (() => void) | undefined;

    getCurrentWebview()
      .onDragDropEvent((evento) => {
        const carga = evento.payload;
        if (carga.type === "enter" || carga.type === "over") {
          setSoltando(true);
        } else if (carga.type === "leave") {
          setSoltando(false);
        } else if (carga.type === "drop") {
          setSoltando(false);
          const validas = carga.paths.filter(admitido);
          const otras = carga.paths.filter((p) => !admitido(p));
          const { avisar, importar } = useBiblioteca.getState();
          if (otras.length) {
            avisar(`Formato no admitido: ${otras.map(nombreDe).join(", ")}`);
          }
          if (validas.length) void importar(validas);
        }
      })
      .then((fn) => {
        // StrictMode desmonta antes de que la promesa resuelva: se suelta ya.
        if (activo) soltar = fn;
        else fn();
      })
      .catch(() => {});

    return () => {
      activo = false;
      soltar?.();
    };
  }, []);

  return soltando;
}
