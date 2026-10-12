// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useBiblioteca } from "../estado/biblioteca";
import { useNavegacion } from "../estado/navegacion";
import icono from "../assets/icono-64.png";
import "./BarraTitulo.css";

/** En `npm run dev` dentro del navegador no hay ventana nativa que controlar. */
const EN_TAURI = isTauri();

/**
 * Barra de título propia (la ventana va sin decoraciones del sistema).
 * Toda la franja es zona de arrastre salvo los botones; el doble clic
 * maximiza o restaura (lo hace Tauri a través de data-tauri-drag-region).
 */
export function BarraTitulo() {
  const ruta = useNavegacion((s) => s.ruta);
  const maximizada = useMaximizada();
  const nombre = useBiblioteca((s) =>
    ruta.vista === "schematic" ? s.entradas.find((e) => e.id === ruta.id)?.nombre : undefined,
  );

  const lugar = ruta.vista === "schematic" ? (nombre ?? ruta.id) : null;

  return (
    <header className="barra-titulo" data-tauri-drag-region>
      <div className="barra-titulo__marca" data-tauri-drag-region>
        <img className="barra-titulo__icono" src={icono} alt="" width={16} height={16} />
        <span className="barra-titulo__nombre rotulo">Litematica3D</span>
      </div>
      <p className="barra-titulo__lugar" data-tauri-drag-region>
        <span>Biblioteca</span>
        {lugar && (
          <>
            <span className="barra-titulo__separador" aria-hidden="true">/</span>
            <span className="barra-titulo__actual">{lugar}</span>
          </>
        )}
      </p>
      {EN_TAURI && <BotonesVentana maximizada={maximizada} />}
    </header>
  );
}

function useMaximizada(): boolean {
  const [maximizada, setMaximizada] = useState(false);
  useEffect(() => {
    if (!EN_TAURI) return;
    const ventana = getCurrentWindow();
    let activo = true;
    let soltar: (() => void) | undefined;
    const actualizar = () => {
      ventana.isMaximized().then((m) => activo && setMaximizada(m));
    };
    actualizar();
    ventana.onResized(actualizar).then((fn) => {
      if (activo) soltar = fn;
      else fn();
    });
    return () => {
      activo = false;
      soltar?.();
    };
  }, []);
  return maximizada;
}

function BotonesVentana({ maximizada }: { maximizada: boolean }) {
  const ventana = getCurrentWindow();
  return (
    <div className="barra-titulo__botones">
      <button
        type="button"
        className="boton-ventana"
        aria-label="Minimizar"
        title="Minimizar"
        onClick={() => ventana.minimize()}
      >
        <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
          <path d="M0 5.5H10" />
        </svg>
      </button>
      <button
        type="button"
        className="boton-ventana"
        aria-label={maximizada ? "Restaurar" : "Maximizar"}
        title={maximizada ? "Restaurar" : "Maximizar"}
        onClick={() => ventana.toggleMaximize()}
      >
        <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
          {maximizada ? (
            <>
              <rect x="0.5" y="2.5" width="7" height="7" />
              <path d="M2.5 2.5V0.5H9.5V7.5H7.5" />
            </>
          ) : (
            <rect x="0.5" y="0.5" width="9" height="9" />
          )}
        </svg>
      </button>
      <button
        type="button"
        className="boton-ventana boton-ventana--cerrar"
        aria-label="Cerrar"
        title="Cerrar"
        onClick={() => ventana.close()}
      >
        <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
          <path className="boton-ventana__aspa" d="M0.5 0.5L9.5 9.5M9.5 0.5L0.5 9.5" />
        </svg>
      </button>
    </div>
  );
}
