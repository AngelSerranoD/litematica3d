// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useBiblioteca } from "../estado/biblioteca";
import icono from "../assets/icono-64.png";
import "./BibliotecaVacia.css";

/**
 * Biblioteca sin schematics: toda la vista es la zona de soltar. Mientras
 * se arrastran archivos sobre la ventana, el filete pasa de discontinuo a
 * continuo y el texto dice qué va a pasar al soltar.
 */
export function BibliotecaVacia({ soltando }: { soltando: boolean }) {
  const elegirEImportar = useBiblioteca((s) => s.elegirEImportar);

  return (
    <div className="vacia" data-soltando={soltando || undefined}>
      <img className="vacia__icono" src={icono} alt="" width={64} height={64} />
      <h2 className="vacia__titulo rotulo" aria-live="polite">
        {soltando ? "Suelta para importar" : "Suelta aquí tus schematics"}
      </h2>
      <p className="vacia__texto">
        Arrastra a la ventana archivos .litematic, .schem, .schematic o .nbt, varios a la vez si
        quieres, o elígelos desde el disco.
      </p>
      <button
        type="button"
        className="boton-plano vacia__boton"
        aria-keyshortcuts="Control+O"
        onClick={() => void elegirEImportar()}
      >
        Elegir archivos
        <kbd>Ctrl O</kbd>
      </button>
    </div>
  );
}
